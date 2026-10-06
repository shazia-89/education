use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};
use anchor_spl::{
    associated_token::get_associated_token_address_with_program_id,
    token_interface::{Mint, TokenAccount},
};
use litesvm::LiteSVM;
use solana_keypair::Keypair;
use solana_message::{AccountMeta, Instruction, Message};
use solana_signer::Signer;
use solana_transaction::Transaction;
use std::{fs, path::PathBuf};

const DECIMALS: u8 = 6;
const INITIAL_MINT: u64 = 1_000_000;
const TRANSFER_AMOUNT: u64 = 250_000;
const BURN_AMOUNT: u64 = 100_000;

fn program_bytes() -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/deploy/solana_level_1_token_starter.so");
    fs::read(&path).unwrap_or_else(|error| {
        panic!(
            "Build the program with `anchor build` before running tests. Could not read {}: {error}",
            path.display()
        )
    })
}

fn new_svm() -> LiteSVM {
    let mut svm = LiteSVM::new();
    svm.add_program(
        solana_level_1_token_starter::ID,
        &program_bytes(),
    )
    .expect("program must load");
    svm
}

fn fund(svm: &mut LiteSVM, keypairs: &[&Keypair]) {
    for keypair in keypairs {
        svm.airdrop(&keypair.pubkey(), 2_000_000_000)
            .expect("airdrop must succeed");
    }
}

fn send(
    svm: &mut LiteSVM,
    payer: &Keypair,
    signers: &[&Keypair],
    instruction: Instruction,
) {
    let message = Message::new(&[instruction], Some(&payer.pubkey()));
    let transaction = Transaction::new(signers, message, svm.latest_blockhash());
    svm.send_transaction(transaction)
        .expect("transaction must succeed");
}

fn send_expect_error(
    svm: &mut LiteSVM,
    payer: &Keypair,
    signers: &[&Keypair],
    instruction: Instruction,
) {
    let message = Message::new(&[instruction], Some(&payer.pubkey()));
    let transaction = Transaction::new(signers, message, svm.latest_blockhash());
    assert!(
        svm.send_transaction(transaction).is_err(),
        "transaction was expected to fail"
    );
}

fn create_token(
    svm: &mut LiteSVM,
    payer: &Keypair,
    authority: &Keypair,
    mint: &Keypair,
    token_program: anchor_lang::prelude::Pubkey,
) {
    let accounts = solana_level_1_token_starter::accounts::CreateToken {
        payer: payer.pubkey(),
        authority: authority.pubkey(),
        mint: mint.pubkey(),
        token_program,
        system_program: anchor_lang::system_program::ID,
    };
    let instruction = Instruction {
        program_id: solana_level_1_token_starter::ID,
        accounts: accounts
            .to_account_metas(None)
            .into_iter()
            .map(|meta| AccountMeta {
                pubkey: meta.pubkey,
                is_signer: meta.is_signer,
                is_writable: meta.is_writable,
            })
            .collect(),
        data: solana_level_1_token_starter::instruction::CreateToken { decimals: DECIMALS }
            .data(),
    };
    send(svm, payer, &[payer, authority, mint], instruction);
}

fn create_token_account(
    svm: &mut LiteSVM,
    payer: &Keypair,
    owner: &Keypair,
    mint: &Keypair,
    token_program: anchor_lang::prelude::Pubkey,
) -> anchor_lang::prelude::Pubkey {
    let token_account = get_associated_token_address_with_program_id(
        &owner.pubkey(),
        &mint.pubkey(),
        &token_program,
    );
    let accounts = solana_level_1_token_starter::accounts::CreateTokenAccount {
        payer: payer.pubkey(),
        owner: owner.pubkey(),
        mint: mint.pubkey(),
        token_account,
        token_program,
        associated_token_program: anchor_spl::associated_token::ID,
        system_program: anchor_lang::system_program::ID,
    };
    let instruction = Instruction {
        program_id: solana_level_1_token_starter::ID,
        accounts: accounts
            .to_account_metas(None)
            .into_iter()
            .map(|meta| AccountMeta {
                pubkey: meta.pubkey,
                is_signer: meta.is_signer,
                is_writable: meta.is_writable,
            })
            .collect(),
        data: solana_level_1_token_starter::instruction::CreateTokenAccount {}.data(),
    };
    send(svm, payer, &[payer], instruction);
    token_account
}

fn mint_tokens(
    svm: &mut LiteSVM,
    authority: &Keypair,
    mint: anchor_lang::prelude::Pubkey,
    destination: anchor_lang::prelude::Pubkey,
    token_program: anchor_lang::prelude::Pubkey,
    amount: u64,
) {
    let accounts = solana_level_1_token_starter::accounts::MintTokens {
        authority: authority.pubkey(),
        mint,
        destination,
        token_program,
    };
    let instruction = Instruction {
        program_id: solana_level_1_token_starter::ID,
        accounts: accounts
            .to_account_metas(None)
            .into_iter()
            .map(|meta| AccountMeta {
                pubkey: meta.pubkey,
                is_signer: meta.is_signer,
                is_writable: meta.is_writable,
            })
            .collect(),
        data: solana_level_1_token_starter::instruction::MintTokens { amount }.data(),
    };
    send(svm, authority, &[authority], instruction);
}

fn transfer_tokens(
    svm: &mut LiteSVM,
    authority: &Keypair,
    mint: anchor_lang::prelude::Pubkey,
    source: anchor_lang::prelude::Pubkey,
    destination: anchor_lang::prelude::Pubkey,
    token_program: anchor_lang::prelude::Pubkey,
    amount: u64,
) {
    let accounts = solana_level_1_token_starter::accounts::TransferTokens {
        authority: authority.pubkey(),
        mint,
        source,
        destination,
        token_program,
    };
    let instruction = Instruction {
        program_id: solana_level_1_token_starter::ID,
        accounts: accounts
            .to_account_metas(None)
            .into_iter()
            .map(|meta| AccountMeta {
                pubkey: meta.pubkey,
                is_signer: meta.is_signer,
                is_writable: meta.is_writable,
            })
            .collect(),
        data: solana_level_1_token_starter::instruction::TransferTokens { amount }.data(),
    };
    send(svm, authority, &[authority], instruction);
}

fn burn_tokens(
    svm: &mut LiteSVM,
    authority: &Keypair,
    mint: anchor_lang::prelude::Pubkey,
    source: anchor_lang::prelude::Pubkey,
    token_program: anchor_lang::prelude::Pubkey,
    amount: u64,
) {
    let accounts = solana_level_1_token_starter::accounts::BurnTokens {
        authority: authority.pubkey(),
        mint,
        source,
        token_program,
    };
    let instruction = Instruction {
        program_id: solana_level_1_token_starter::ID,
        accounts: accounts
            .to_account_metas(None)
            .into_iter()
            .map(|meta| AccountMeta {
                pubkey: meta.pubkey,
                is_signer: meta.is_signer,
                is_writable: meta.is_writable,
            })
            .collect(),
        data: solana_level_1_token_starter::instruction::BurnTokens { amount }.data(),
    };
    send(svm, authority, &[authority], instruction);
}

fn token_mint(svm: &LiteSVM, address: &anchor_lang::prelude::Pubkey) -> Mint {
    let account = svm.get_account(address).expect("mint must exist");
    let mut data: &[u8] = &account.data;
    Mint::try_deserialize(&mut data).expect("mint account must deserialize")
}

fn token_account(svm: &LiteSVM, address: &anchor_lang::prelude::Pubkey) -> TokenAccount {
    let account = svm
        .get_account(address)
        .expect("token account must exist");
    let mut data: &[u8] = &account.data;
    TokenAccount::try_deserialize(&mut data).expect("token account must deserialize")
}

#[test]
fn creates_token_2022_mint() {
    let token_program = anchor_spl::token_2022::ID;
    let mut svm = new_svm();
    let payer = Keypair::new();
    let authority = Keypair::new();
    let mint = Keypair::new();
    fund(&mut svm, &[&payer]);

    create_token(&mut svm, &payer, &authority, &mint, token_program);

    let mint_account = svm.get_account(&mint.pubkey()).expect("mint must exist");
    let mint_state = token_mint(&svm, &mint.pubkey());
    assert_eq!(mint_account.owner, token_program);
    assert_eq!(mint_state.decimals, DECIMALS);
    assert_eq!(mint_state.mint_authority, Some(authority.pubkey()));
    assert_eq!(mint_state.supply, 0);
}

#[test]
fn creates_token_account_with_expected_owner_mint_and_program() {
    let token_program = anchor_spl::token_2022::ID;
    let mut svm = new_svm();
    let payer = Keypair::new();
    let authority = Keypair::new();
    let owner = Keypair::new();
    let mint = Keypair::new();
    fund(&mut svm, &[&payer, &owner]);
    create_token(&mut svm, &payer, &authority, &mint, token_program);

    let token_account = create_token_account(&mut svm, &payer, &owner, &mint, token_program);
    let account = svm
        .get_account(&token_account)
        .expect("token account must exist");
    let state = token_account(&svm, &token_account);

    assert_eq!(account.owner, token_program);
    assert_eq!(state.owner, owner.pubkey());
    assert_eq!(state.mint, mint.pubkey());
    assert_eq!(state.amount, 0);
}

#[test]
fn mints_tokens_and_updates_recipient_balance_and_supply() {
    let token_program = anchor_spl::token_2022::ID;
    let mut svm = new_svm();
    let payer = Keypair::new();
    let authority = Keypair::new();
    let recipient = Keypair::new();
    let mint = Keypair::new();
    fund(&mut svm, &[&payer, &recipient]);
    create_token(&mut svm, &payer, &authority, &mint, token_program);
    let destination = create_token_account(&mut svm, &payer, &recipient, &mint, token_program);

    mint_tokens(
        &mut svm,
        &authority,
        mint.pubkey(),
        destination,
        token_program,
        INITIAL_MINT,
    );

    assert_eq!(token_account(&svm, &destination).amount, INITIAL_MINT);
    assert_eq!(token_mint(&svm, &mint.pubkey()).supply, INITIAL_MINT);
}

#[test]
fn transfers_tokens_and_keeps_supply_unchanged() {
    let token_program = anchor_spl::token_2022::ID;
    let mut svm = new_svm();
    let payer = Keypair::new();
    let authority = Keypair::new();
    let recipient = Keypair::new();
    let mint = Keypair::new();
    fund(&mut svm, &[&payer, &recipient]);
    create_token(&mut svm, &payer, &authority, &mint, token_program);
    let source = create_token_account(&mut svm, &payer, &authority, &mint, token_program);
    let destination = create_token_account(&mut svm, &payer, &recipient, &mint, token_program);
    mint_tokens(
        &mut svm,
        &authority,
        mint.pubkey(),
        source,
        token_program,
        INITIAL_MINT,
    );

    let supply_before = token_mint(&svm, &mint.pubkey()).supply;
    let source_before = token_account(&svm, &source).amount;
    let destination_before = token_account(&svm, &destination).amount;

    transfer_tokens(
        &mut svm,
        &authority,
        mint.pubkey(),
        source,
        destination,
        token_program,
        TRANSFER_AMOUNT,
    );

    assert_eq!(token_account(&svm, &source).amount, source_before - TRANSFER_AMOUNT);
    assert_eq!(
        token_account(&svm, &destination).amount,
        destination_before + TRANSFER_AMOUNT
    );
    assert_eq!(token_mint(&svm, &mint.pubkey()).supply, supply_before);
}

#[test]
fn rejects_zero_amount_for_mint_and_transfer() {
    let token_program = anchor_spl::token_2022::ID;
    let mut svm = new_svm();
    let payer = Keypair::new();
    let authority = Keypair::new();
    let recipient = Keypair::new();
    let mint = Keypair::new();
    fund(&mut svm, &[&payer, &recipient]);
    create_token(&mut svm, &payer, &authority, &mint, token_program);
    let source = create_token_account(&mut svm, &payer, &authority, &mint, token_program);
    let destination = create_token_account(&mut svm, &payer, &recipient, &mint, token_program);

    let mint_accounts = solana_level_1_token_starter::accounts::MintTokens {
        authority: authority.pubkey(),
        mint: mint.pubkey(),
        destination: source,
        token_program,
    };
    let mint_ix = Instruction {
        program_id: solana_level_1_token_starter::ID,
        accounts: mint_accounts
            .to_account_metas(None)
            .into_iter()
            .map(|meta| AccountMeta {
                pubkey: meta.pubkey,
                is_signer: meta.is_signer,
                is_writable: meta.is_writable,
            })
            .collect(),
        data: solana_level_1_token_starter::instruction::MintTokens { amount: 0 }.data(),
    };
    send_expect_error(&mut svm, &authority, &[&authority], mint_ix);

    let transfer_accounts = solana_level_1_token_starter::accounts::TransferTokens {
        authority: authority.pubkey(),
        mint: mint.pubkey(),
        source,
        destination,
        token_program,
    };
    let transfer_ix = Instruction {
        program_id: solana_level_1_token_starter::ID,
        accounts: transfer_accounts
            .to_account_metas(None)
            .into_iter()
            .map(|meta| AccountMeta {
                pubkey: meta.pubkey,
                is_signer: meta.is_signer,
                is_writable: meta.is_writable,
            })
            .collect(),
        data: solana_level_1_token_starter::instruction::TransferTokens { amount: 0 }.data(),
    };
    send_expect_error(&mut svm, &authority, &[&authority], transfer_ix);
}

#[test]
fn rejects_wrong_mint_authority() {
    let token_program = anchor_spl::token_2022::ID;
    let mut svm = new_svm();
    let payer = Keypair::new();
    let authority = Keypair::new();
    let wrong_authority = Keypair::new();
    let recipient = Keypair::new();
    let mint = Keypair::new();
    fund(&mut svm, &[&payer, &recipient, &wrong_authority]);
    create_token(&mut svm, &payer, &authority, &mint, token_program);
    let destination = create_token_account(&mut svm, &payer, &recipient, &mint, token_program);

    let accounts = solana_level_1_token_starter::accounts::MintTokens {
        authority: wrong_authority.pubkey(),
        mint: mint.pubkey(),
        destination,
        token_program,
    };
    let ix = Instruction {
        program_id: solana_level_1_token_starter::ID,
        accounts: accounts
            .to_account_metas(None)
            .into_iter()
            .map(|meta| AccountMeta {
                pubkey: meta.pubkey,
                is_signer: meta.is_signer,
                is_writable: meta.is_writable,
            })
            .collect(),
        data: solana_level_1_token_starter::instruction::MintTokens { amount: 1 }.data(),
    };
    send_expect_error(&mut svm, &wrong_authority, &[&wrong_authority], ix);
}

#[test]
fn rejects_destination_from_a_different_mint() {
    let token_program = anchor_spl::token_2022::ID;
    let mut svm = new_svm();
    let payer = Keypair::new();
    let authority = Keypair::new();
    let recipient = Keypair::new();
    let mint_a = Keypair::new();
    let mint_b = Keypair::new();
    fund(&mut svm, &[&payer, &recipient]);
    create_token(&mut svm, &payer, &authority, &mint_a, token_program);
    create_token(&mut svm, &payer, &authority, &mint_b, token_program);
    let destination_for_a =
        create_token_account(&mut svm, &payer, &recipient, &mint_a, token_program);

    let accounts = solana_level_1_token_starter::accounts::MintTokens {
        authority: authority.pubkey(),
        mint: mint_b.pubkey(),
        destination: destination_for_a,
        token_program,
    };
    let ix = Instruction {
        program_id: solana_level_1_token_starter::ID,
        accounts: accounts
            .to_account_metas(None)
            .into_iter()
            .map(|meta| AccountMeta {
                pubkey: meta.pubkey,
                is_signer: meta.is_signer,
                is_writable: meta.is_writable,
            })
            .collect(),
        data: solana_level_1_token_starter::instruction::MintTokens { amount: 1 }.data(),
    };
    send_expect_error(&mut svm, &authority, &[&authority], ix);
}

#[test]
fn rejects_identical_transfer_source_and_destination() {
    let token_program = anchor_spl::token_2022::ID;
    let mut svm = new_svm();
    let payer = Keypair::new();
    let authority = Keypair::new();
    let mint = Keypair::new();
    fund(&mut svm, &[&payer]);
    create_token(&mut svm, &payer, &authority, &mint, token_program);
    let source = create_token_account(&mut svm, &payer, &authority, &mint, token_program);

    let accounts = solana_level_1_token_starter::accounts::TransferTokens {
        authority: authority.pubkey(),
        mint: mint.pubkey(),
        source,
        destination: source,
        token_program,
    };
    let ix = Instruction {
        program_id: solana_level_1_token_starter::ID,
        accounts: accounts
            .to_account_metas(None)
            .into_iter()
            .map(|meta| AccountMeta {
                pubkey: meta.pubkey,
                is_signer: meta.is_signer,
                is_writable: meta.is_writable,
            })
            .collect(),
        data: solana_level_1_token_starter::instruction::TransferTokens { amount: 1 }.data(),
    };
    send_expect_error(&mut svm, &authority, &[&authority], ix);
}

#[test]
fn burns_tokens_and_reduces_balance_and_supply() {
    let token_program = anchor_spl::token_2022::ID;
    let mut svm = new_svm();
    let payer = Keypair::new();
    let authority = Keypair::new();
    let mint = Keypair::new();
    fund(&mut svm, &[&payer]);
    create_token(&mut svm, &payer, &authority, &mint, token_program);
    let source = create_token_account(&mut svm, &payer, &authority, &mint, token_program);
    mint_tokens(&mut svm, &authority, mint.pubkey(), source, token_program, INITIAL_MINT);

    let balance_before = token_account(&svm, &source).amount;
    let supply_before = token_mint(&svm, &mint.pubkey()).supply;

    burn_tokens(
        &mut svm,
        &authority,
        mint.pubkey(),
        source,
        token_program,
        BURN_AMOUNT,
    );

    assert_eq!(token_account(&svm, &source).amount, balance_before - BURN_AMOUNT);
    assert_eq!(token_mint(&svm, &mint.pubkey()).supply, supply_before - BURN_AMOUNT);
}

#[test]
fn rejects_zero_amount_for_burn_and_preserves_state() {
    let token_program = anchor_spl::token_2022::ID;
    let mut svm = new_svm();
    let payer = Keypair::new();
    let authority = Keypair::new();
    let mint = Keypair::new();
    fund(&mut svm, &[&payer]);
    create_token(&mut svm, &payer, &authority, &mint, token_program);
    let source = create_token_account(&mut svm, &payer, &authority, &mint, token_program);
    mint_tokens(&mut svm, &authority, mint.pubkey(), source, token_program, INITIAL_MINT);

    let balance_before = token_account(&svm, &source).amount;
    let supply_before = token_mint(&svm, &mint.pubkey()).supply;
    let accounts = solana_level_1_token_starter::accounts::BurnTokens {
        authority: authority.pubkey(), mint: mint.pubkey(), source, token_program,
    };
    let ix = Instruction {
        program_id: solana_level_1_token_starter::ID,
        accounts: accounts.to_account_metas(None).into_iter().map(|meta| AccountMeta {
            pubkey: meta.pubkey, is_signer: meta.is_signer, is_writable: meta.is_writable,
        }).collect(),
        data: solana_level_1_token_starter::instruction::BurnTokens { amount: 0 }.data(),
    };
    send_expect_error(&mut svm, &authority, &[&authority], ix);
    assert_eq!(token_account(&svm, &source).amount, balance_before);
    assert_eq!(token_mint(&svm, &mint.pubkey()).supply, supply_before);
}

#[test]
fn rejects_wrong_burn_authority_and_preserves_state() {
    let token_program = anchor_spl::token_2022::ID;
    let mut svm = new_svm();
    let payer = Keypair::new();
    let authority = Keypair::new();
    let wrong_authority = Keypair::new();
    let mint = Keypair::new();
    fund(&mut svm, &[&payer, &wrong_authority]);
    create_token(&mut svm, &payer, &authority, &mint, token_program);
    let source = create_token_account(&mut svm, &payer, &authority, &mint, token_program);
    mint_tokens(&mut svm, &authority, mint.pubkey(), source, token_program, INITIAL_MINT);

    let balance_before = token_account(&svm, &source).amount;
    let supply_before = token_mint(&svm, &mint.pubkey()).supply;
    let accounts = solana_level_1_token_starter::accounts::BurnTokens {
        authority: wrong_authority.pubkey(), mint: mint.pubkey(), source, token_program,
    };
    let ix = Instruction {
        program_id: solana_level_1_token_starter::ID,
        accounts: accounts.to_account_metas(None).into_iter().map(|meta| AccountMeta {
            pubkey: meta.pubkey, is_signer: meta.is_signer, is_writable: meta.is_writable,
        }).collect(),
        data: solana_level_1_token_starter::instruction::BurnTokens { amount: 1 }.data(),
    };
    send_expect_error(&mut svm, &wrong_authority, &[&wrong_authority], ix);
    assert_eq!(token_account(&svm, &source).amount, balance_before);
    assert_eq!(token_mint(&svm, &mint.pubkey()).supply, supply_before);
}

#[test]
fn rejects_wrong_mint_and_preserves_state() {
    let token_program = anchor_spl::token_2022::ID;
    let mut svm = new_svm();
    let payer = Keypair::new();
    let authority = Keypair::new();
    let mint_a = Keypair::new();
    let mint_b = Keypair::new();
    fund(&mut svm, &[&payer]);
    create_token(&mut svm, &payer, &authority, &mint_a, token_program);
    create_token(&mut svm, &payer, &authority, &mint_b, token_program);
    let source = create_token_account(&mut svm, &payer, &authority, &mint_a, token_program);
    mint_tokens(&mut svm, &authority, mint_a.pubkey(), source, token_program, INITIAL_MINT);

    let balance_before = token_account(&svm, &source).amount;
    let supply_a_before = token_mint(&svm, &mint_a.pubkey()).supply;
    let accounts = solana_level_1_token_starter::accounts::BurnTokens {
        authority: authority.pubkey(), mint: mint_b.pubkey(), source, token_program,
    };
    let ix = Instruction {
        program_id: solana_level_1_token_starter::ID,
        accounts: accounts.to_account_metas(None).into_iter().map(|meta| AccountMeta {
            pubkey: meta.pubkey, is_signer: meta.is_signer, is_writable: meta.is_writable,
        }).collect(),
        data: solana_level_1_token_starter::instruction::BurnTokens { amount: 1 }.data(),
    };
    send_expect_error(&mut svm, &authority, &[&authority], ix);
    assert_eq!(token_account(&svm, &source).amount, balance_before);
    assert_eq!(token_mint(&svm, &mint_a.pubkey()).supply, supply_a_before);
}

#[test]
fn rejects_insufficient_burn_balance_and_preserves_state() {
    let token_program = anchor_spl::token_2022::ID;
    let mut svm = new_svm();
    let payer = Keypair::new();
    let authority = Keypair::new();
    let mint = Keypair::new();
    fund(&mut svm, &[&payer]);
    create_token(&mut svm, &payer, &authority, &mint, token_program);
    let source = create_token_account(&mut svm, &payer, &authority, &mint, token_program);
    mint_tokens(&mut svm, &authority, mint.pubkey(), source, token_program, INITIAL_MINT);

    burn_tokens(&mut svm, &authority, mint.pubkey(), source, token_program, 1);
    let balance_after_success = token_account(&svm, &source).amount;
    let supply_after_success = token_mint(&svm, &mint.pubkey()).supply;

    let accounts = solana_level_1_token_starter::accounts::BurnTokens {
        authority: authority.pubkey(), mint: mint.pubkey(), source, token_program,
    };
    let ix = Instruction {
        program_id: solana_level_1_token_starter::ID,
        accounts: accounts.to_account_metas(None).into_iter().map(|meta| AccountMeta {
            pubkey: meta.pubkey, is_signer: meta.is_signer, is_writable: meta.is_writable,
        }).collect(),
        data: solana_level_1_token_starter::instruction::BurnTokens { amount: balance_after_success + 1 }.data(),
    };
    send_expect_error(&mut svm, &authority, &[&authority], ix);
    assert_eq!(token_account(&svm, &source).amount, balance_after_success);
    assert_eq!(token_mint(&svm, &mint.pubkey()).supply, supply_after_success);
}
