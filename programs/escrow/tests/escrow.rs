use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};
use anchor_spl::token_interface::{Mint, TokenAccount};
use litesvm::LiteSVM;
use solana_keypair::Keypair;
use solana_message::{AccountMeta, Instruction, Message};
use solana_signer::Signer;
use solana_transaction::Transaction;
use std::{fs, path::PathBuf};

const DECIMALS: u8 = 6;
const INITIAL_MINT: u64 = 1_000_000;
const DEAL_AMOUNT: u64 = 100_000;

fn program_bytes(path: &str) -> Vec<u8> {
    fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path)).unwrap()
}

fn new_svm() -> LiteSVM {
    let mut svm = LiteSVM::new();
    svm.add_program(solana_level_1_token_starter::ID, &program_bytes("../../target/deploy/solana_level_1_token_starter.so")).unwrap();
    svm.add_program(escrow::ID, &program_bytes("../../target/deploy/escrow.so")).unwrap();
    svm
}

fn fund(svm: &mut LiteSVM, keys: &[&Keypair]) {
    for key in keys { svm.airdrop(&key.pubkey(), 5_000_000_000).unwrap(); }
}

fn send(svm: &mut LiteSVM, payer: &Keypair, signers: &[&Keypair], ix: Instruction) {
    let msg = Message::new(&[ix], Some(&payer.pubkey()));
    let tx = Transaction::new(signers, msg, svm.latest_blockhash());
    svm.send_transaction(tx).unwrap();
}

fn send_err(svm: &mut LiteSVM, payer: &Keypair, signers: &[&Keypair], ix: Instruction) {
    let msg = Message::new(&[ix], Some(&payer.pubkey()));
    let tx = Transaction::new(signers, msg, svm.latest_blockhash());
    assert!(svm.send_transaction(tx).is_err());
}

fn mint_setup(svm: &mut LiteSVM, payer: &Keypair, sender: &Keypair, receiver: &Keypair, mint: &Keypair) -> (anchor_lang::prelude::Pubkey, anchor_lang::prelude::Pubkey) {
    let token_program = anchor_spl::token_2022::ID;
    let create = solana_level_1_token_starter::accounts::CreateToken { payer: payer.pubkey(), authority: sender.pubkey(), mint: mint.pubkey(), token_program, system_program: anchor_lang::system_program::ID };
    send(svm, payer, &[payer, sender, mint], Instruction { program_id: solana_level_1_token_starter::ID, accounts: create.to_account_metas(None).into_iter().map(|m| AccountMeta { pubkey:m.pubkey, is_signer:m.is_signer, is_writable:m.is_writable }).collect(), data: solana_level_1_token_starter::instruction::CreateToken { decimals: DECIMALS }.data() });
    let sender_ata = anchor_spl::associated_token::get_associated_token_address_with_program_id(&sender.pubkey(), &mint.pubkey(), &token_program);
    let receiver_ata = anchor_spl::associated_token::get_associated_token_address_with_program_id(&receiver.pubkey(), &mint.pubkey(), &token_program);
    for (owner, ata) in [(sender, sender_ata), (receiver, receiver_ata)] {
        let a = solana_level_1_token_starter::accounts::CreateTokenAccount { payer:payer.pubkey(), owner:owner.pubkey(), mint:mint.pubkey(), token_account:ata, token_program, associated_token_program:anchor_spl::associated_token::ID, system_program:anchor_lang::system_program::ID };
        send(svm, payer, &[payer], Instruction { program_id: solana_level_1_token_starter::ID, accounts:a.to_account_metas(None).into_iter().map(|m| AccountMeta{pubkey:m.pubkey,is_signer:m.is_signer,is_writable:m.is_writable}).collect(), data: solana_level_1_token_starter::instruction::CreateTokenAccount{}.data() });
    }
    let m = solana_level_1_token_starter::accounts::MintTokens { authority:sender.pubkey(), mint:mint.pubkey(), destination:sender_ata, token_program };
    send(svm, sender, &[sender], Instruction { program_id:solana_level_1_token_starter::ID, accounts:m.to_account_metas(None).into_iter().map(|m|AccountMeta{pubkey:m.pubkey,is_signer:m.is_signer,is_writable:m.is_writable}).collect(), data:solana_level_1_token_starter::instruction::MintTokens{amount:INITIAL_MINT}.data() });
    (sender_ata, receiver_ata)
}

fn token_account(svm: &LiteSVM, key: &anchor_lang::prelude::Pubkey) -> TokenAccount { let a=svm.get_account(key).unwrap(); let mut d:&[u8]=&a.data; TokenAccount::try_deserialize(&mut d).unwrap() }
fn mint_state(svm: &LiteSVM, key: &anchor_lang::prelude::Pubkey) -> Mint { let a=svm.get_account(key).unwrap(); let mut d:&[u8]=&a.data; Mint::try_deserialize(&mut d).unwrap() }

fn state_pda(sender: &Keypair, deal_id: u64) -> anchor_lang::prelude::Pubkey { anchor_lang::prelude::Pubkey::find_program_address(&[b"escrow", sender.pubkey().as_ref(), &deal_id.to_le_bytes()], &escrow::ID).0 }

#[test]
fn release_moves_exact_amount_and_closes_state_and_vault() {
    let mut svm=new_svm(); let payer=Keypair::new(); let sender=Keypair::new(); let receiver=Keypair::new(); let mint=Keypair::new(); fund(&mut svm,&[&payer,&sender,&receiver]);
    let (sender_ata, receiver_ata)=mint_setup(&mut svm,&payer,&sender,&receiver,&mint); let deal_id=7; let state=state_pda(&sender,deal_id); let vault=anchor_spl::associated_token::get_associated_token_address_with_program_id(&state,&mint.pubkey(),&anchor_spl::token_2022::ID);
    let i=escrow::accounts::Initialize{sender:sender.pubkey(),receiver:receiver.pubkey(),mint:mint.pubkey(),state,vault,token_program:anchor_spl::token_2022::ID,associated_token_program:anchor_spl::associated_token::ID,system_program:anchor_lang::system_program::ID};
    send(&mut svm,&sender,&[&sender],Instruction{program_id:escrow::ID,accounts:i.to_account_metas(None).into_iter().map(|m|AccountMeta{pubkey:m.pubkey,is_signer:m.is_signer,is_writable:m.is_writable}).collect(),data:escrow::instruction::Initialize{deal_id,amount:DEAL_AMOUNT}.data()});
    let d=escrow::accounts::Deposit{sender:sender.pubkey(),state,mint:mint.pubkey(),source:sender_ata,vault,token_program:anchor_spl::token_2022::ID};
    send(&mut svm,&sender,&[&sender],Instruction{program_id:escrow::ID,accounts:d.to_account_metas(None).into_iter().map(|m|AccountMeta{pubkey:m.pubkey,is_signer:m.is_signer,is_writable:m.is_writable}).collect(),data:escrow::instruction::Deposit{}.data()});
    assert_eq!(token_account(&svm,&vault).amount,DEAL_AMOUNT); assert_eq!(mint_state(&svm,&mint.pubkey()).supply,INITIAL_MINT);
    let r=escrow::accounts::Release{sender:sender.pubkey(),state,mint:mint.pubkey(),vault,receiver_token_account:receiver_ata,receiver:receiver.pubkey(),token_program:anchor_spl::token_2022::ID};
    send(&mut svm,&sender,&[&sender],Instruction{program_id:escrow::ID,accounts:r.to_account_metas(None).into_iter().map(|m|AccountMeta{pubkey:m.pubkey,is_signer:m.is_signer,is_writable:m.is_writable}).collect(),data:escrow::instruction::Release{}.data()});
    assert_eq!(token_account(&svm,&sender_ata).amount,INITIAL_MINT-DEAL_AMOUNT); assert_eq!(token_account(&svm,&receiver_ata).amount,DEAL_AMOUNT); assert!(svm.get_account(&state).is_none()); assert!(svm.get_account(&vault).is_none());
    let repeat = escrow::accounts::Release{sender:sender.pubkey(),state,mint:mint.pubkey(),vault,receiver_token_account:receiver_ata,receiver:receiver.pubkey(),token_program:anchor_spl::token_2022::ID};
    let repeat_ix=Instruction{program_id:escrow::ID,accounts:repeat.to_account_metas(None).into_iter().map(|m|AccountMeta{pubkey:m.pubkey,is_signer:m.is_signer,is_writable:m.is_writable}).collect(),data:escrow::instruction::Release{}.data()};
    send_err(&mut svm,&sender,&[&sender],repeat_ix);
}

#[test]
fn cancel_refunds_sender_and_closes_state_and_vault() {
    let mut svm=new_svm(); let payer=Keypair::new(); let sender=Keypair::new(); let receiver=Keypair::new(); let mint=Keypair::new(); fund(&mut svm,&[&payer,&sender,&receiver]);
    let (sender_ata,_)=mint_setup(&mut svm,&payer,&sender,&receiver,&mint); let deal_id=8; let state=state_pda(&sender,deal_id); let vault=anchor_spl::associated_token::get_associated_token_address_with_program_id(&state,&mint.pubkey(),&anchor_spl::token_2022::ID);
    let i=escrow::accounts::Initialize{sender:sender.pubkey(),receiver:receiver.pubkey(),mint:mint.pubkey(),state,vault,token_program:anchor_spl::token_2022::ID,associated_token_program:anchor_spl::associated_token::ID,system_program:anchor_lang::system_program::ID}; send(&mut svm,&sender,&[&sender],Instruction{program_id:escrow::ID,accounts:i.to_account_metas(None).into_iter().map(|m|AccountMeta{pubkey:m.pubkey,is_signer:m.is_signer,is_writable:m.is_writable}).collect(),data:escrow::instruction::Initialize{deal_id,amount:DEAL_AMOUNT}.data()});
    let d=escrow::accounts::Deposit{sender:sender.pubkey(),state,mint:mint.pubkey(),source:sender_ata,vault,token_program:anchor_spl::token_2022::ID}; send(&mut svm,&sender,&[&sender],Instruction{program_id:escrow::ID,accounts:d.to_account_metas(None).into_iter().map(|m|AccountMeta{pubkey:m.pubkey,is_signer:m.is_signer,is_writable:m.is_writable}).collect(),data:escrow::instruction::Deposit{}.data()});
    let c=escrow::accounts::Cancel{sender:sender.pubkey(),state,mint:mint.pubkey(),sender_token_account:sender_ata,vault,token_program:anchor_spl::token_2022::ID}; send(&mut svm,&sender,&[&sender],Instruction{program_id:escrow::ID,accounts:c.to_account_metas(None).into_iter().map(|m|AccountMeta{pubkey:m.pubkey,is_signer:m.is_signer,is_writable:m.is_writable}).collect(),data:escrow::instruction::Cancel{}.data()});
    assert_eq!(token_account(&svm,&sender_ata).amount,INITIAL_MINT); assert!(svm.get_account(&state).is_none()); assert!(svm.get_account(&vault).is_none());
}

#[test]
fn rejects_zero_amount_and_repeated_finalize() {
    let mut svm=new_svm(); let payer=Keypair::new(); let sender=Keypair::new(); let receiver=Keypair::new(); let mint=Keypair::new(); fund(&mut svm,&[&payer,&sender,&receiver]);
    let token_program=anchor_spl::token_2022::ID; let state=state_pda(&sender,1); let vault=anchor_spl::associated_token::get_associated_token_address_with_program_id(&state,&mint.pubkey(),&token_program);
    let i=escrow::accounts::Initialize{sender:sender.pubkey(),receiver:receiver.pubkey(),mint:mint.pubkey(),state,vault,token_program,associated_token_program:anchor_spl::associated_token::ID,system_program:anchor_lang::system_program::ID};
    let ix=Instruction{program_id:escrow::ID,accounts:i.to_account_metas(None).into_iter().map(|m|AccountMeta{pubkey:m.pubkey,is_signer:m.is_signer,is_writable:m.is_writable}).collect(),data:escrow::instruction::Initialize{deal_id:1,amount:0}.data()}; send_err(&mut svm,&sender,&[&sender],ix);
}

#[test]
fn rejects_wrong_authority_and_preserves_state() {
    let mut svm=new_svm(); let payer=Keypair::new(); let sender=Keypair::new(); let attacker=Keypair::new(); let receiver=Keypair::new(); let mint=Keypair::new(); fund(&mut svm,&[&payer,&sender,&attacker,&receiver]);
    let (sender_ata,_)=mint_setup(&mut svm,&payer,&sender,&receiver,&mint); let deal_id=20; let state=state_pda(&sender,deal_id); let vault=anchor_spl::associated_token::get_associated_token_address_with_program_id(&state,&mint.pubkey(),&anchor_spl::token_2022::ID);
    let i=escrow::accounts::Initialize{sender:sender.pubkey(),receiver:receiver.pubkey(),mint:mint.pubkey(),state,vault,token_program:anchor_spl::token_2022::ID,associated_token_program:anchor_spl::associated_token::ID,system_program:anchor_lang::system_program::ID}; send(&mut svm,&sender,&[&sender],Instruction{program_id:escrow::ID,accounts:i.to_account_metas(None).into_iter().map(|m|AccountMeta{pubkey:m.pubkey,is_signer:m.is_signer,is_writable:m.is_writable}).collect(),data:escrow::instruction::Initialize{deal_id,amount:DEAL_AMOUNT}.data()});
    let d=escrow::accounts::Deposit{sender:sender.pubkey(),state,mint:mint.pubkey(),source:sender_ata,vault,token_program:anchor_spl::token_2022::ID}; send(&mut svm,&sender,&[&sender],Instruction{program_id:escrow::ID,accounts:d.to_account_metas(None).into_iter().map(|m|AccountMeta{pubkey:m.pubkey,is_signer:m.is_signer,is_writable:m.is_writable}).collect(),data:escrow::instruction::Deposit{}.data()});
    let before=token_account(&svm,&vault).amount;
    let r=escrow::accounts::Release{sender:sender.pubkey(),state,mint:mint.pubkey(),vault,receiver_token_account:anchor_spl::associated_token::get_associated_token_address_with_program_id(&receiver.pubkey(),&mint.pubkey(),&anchor_spl::token_2022::ID),receiver:receiver.pubkey(),token_program:anchor_spl::token_2022::ID};
    let ix=Instruction{program_id:escrow::ID,accounts:r.to_account_metas(None).into_iter().map(|m|AccountMeta{pubkey:m.pubkey,is_signer:m.is_signer,is_writable:m.is_writable}).collect(),data:escrow::instruction::Release{}.data()}; send_err(&mut svm,&attacker,&[&attacker],ix);
    assert_eq!(token_account(&svm,&vault).amount,before); assert!(svm.get_account(&state).is_some());
}

#[test]
fn rejects_wrong_mint_and_preserves_state() {
    let mut svm=new_svm(); let payer=Keypair::new(); let sender=Keypair::new(); let receiver=Keypair::new(); let mint=Keypair::new(); let wrong_mint=Keypair::new(); fund(&mut svm,&[&payer,&sender,&receiver]);
    let (sender_ata,_)=mint_setup(&mut svm,&payer,&sender,&receiver,&mint);
    let create=solana_level_1_token_starter::accounts::CreateToken{payer:payer.pubkey(),authority:sender.pubkey(),mint:wrong_mint.pubkey(),token_program:anchor_spl::token_2022::ID,system_program:anchor_lang::system_program::ID}; send(&mut svm,&payer,&[&payer,&sender,&wrong_mint],Instruction{program_id:solana_level_1_token_starter::ID,accounts:create.to_account_metas(None).into_iter().map(|m|AccountMeta{pubkey:m.pubkey,is_signer:m.is_signer,is_writable:m.is_writable}).collect(),data:solana_level_1_token_starter::instruction::CreateToken{decimals:DECIMALS}.data()});
    let deal_id=21; let state=state_pda(&sender,deal_id); let vault=anchor_spl::associated_token::get_associated_token_address_with_program_id(&state,&mint.pubkey(),&anchor_spl::token_2022::ID);
    let i=escrow::accounts::Initialize{sender:sender.pubkey(),receiver:receiver.pubkey(),mint:mint.pubkey(),state,vault,token_program:anchor_spl::token_2022::ID,associated_token_program:anchor_spl::associated_token::ID,system_program:anchor_lang::system_program::ID}; send(&mut svm,&sender,&[&sender],Instruction{program_id:escrow::ID,accounts:i.to_account_metas(None).into_iter().map(|m|AccountMeta{pubkey:m.pubkey,is_signer:m.is_signer,is_writable:m.is_writable}).collect(),data:escrow::instruction::Initialize{deal_id,amount:DEAL_AMOUNT}.data()});
    let d=escrow::accounts::Deposit{sender:sender.pubkey(),state,mint:wrong_mint.pubkey(),source:sender_ata,vault,token_program:anchor_spl::token_2022::ID}; let ix=Instruction{program_id:escrow::ID,accounts:d.to_account_metas(None).into_iter().map(|m|AccountMeta{pubkey:m.pubkey,is_signer:m.is_signer,is_writable:m.is_writable}).collect(),data:escrow::instruction::Deposit{}.data()}; send_err(&mut svm,&sender,&[&sender],ix);
    assert_eq!(token_account(&svm,&sender_ata).amount,INITIAL_MINT); assert_eq!(token_account(&svm,&vault).amount,0); assert!(svm.get_account(&state).is_some());
}

#[test]
fn rejects_insufficient_balance_and_preserves_state() {
    let mut svm=new_svm(); let payer=Keypair::new(); let sender=Keypair::new(); let receiver=Keypair::new(); let mint=Keypair::new(); fund(&mut svm,&[&payer,&sender,&receiver]);
    let (sender_ata,_)=mint_setup(&mut svm,&payer,&sender,&receiver,&mint); let deal_id=22; let amount=INITIAL_MINT+1; let state=state_pda(&sender,deal_id); let vault=anchor_spl::associated_token::get_associated_token_address_with_program_id(&state,&mint.pubkey(),&anchor_spl::token_2022::ID);
    let i=escrow::accounts::Initialize{sender:sender.pubkey(),receiver:receiver.pubkey(),mint:mint.pubkey(),state,vault,token_program:anchor_spl::token_2022::ID,associated_token_program:anchor_spl::associated_token::ID,system_program:anchor_lang::system_program::ID}; send(&mut svm,&sender,&[&sender],Instruction{program_id:escrow::ID,accounts:i.to_account_metas(None).into_iter().map(|m|AccountMeta{pubkey:m.pubkey,is_signer:m.is_signer,is_writable:m.is_writable}).collect(),data:escrow::instruction::Initialize{deal_id,amount}.data()});
    let d=escrow::accounts::Deposit{sender:sender.pubkey(),state,mint:mint.pubkey(),source:sender_ata,vault,token_program:anchor_spl::token_2022::ID}; let ix=Instruction{program_id:escrow::ID,accounts:d.to_account_metas(None).into_iter().map(|m|AccountMeta{pubkey:m.pubkey,is_signer:m.is_signer,is_writable:m.is_writable}).collect(),data:escrow::instruction::Deposit{}.data()}; send_err(&mut svm,&sender,&[&sender],ix);
    assert_eq!(token_account(&svm,&sender_ata).amount,INITIAL_MINT); assert_eq!(token_account(&svm,&vault).amount,0); assert!(svm.get_account(&state).is_some());
}
