use {
    anchor_lang::{
        prelude::Pubkey, solana_program::instruction::Instruction,
        system_program::ID as SYSTEM_PROGRAM_ID, InstructionData, ToAccountMetas,
    },
    anchor_spl::{
        associated_token::{self, ID as ASSOCIATED_TOKEN_PROGRAM_ID},
        token::{self, ID as TOKEN_PROGRAM_ID},
    },
    litesvm::LiteSVM,
    solana_keypair::Keypair,
    solana_signer::Signer,
    solana_transaction::Transaction,
};

pub fn create_deposit_ix(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint_x: Pubkey,
    mint_y: Pubkey,
    mint_lp: Pubkey,
    config: Pubkey,
    vault_x: Pubkey,
    vault_y: Pubkey,
) -> Instruction {
    let user = payer.pubkey();

    let user_x = associated_token::get_associated_token_address(&user, &mint_x);
    let user_y = associated_token::get_associated_token_address(&user, &mint_y);

    for (mint, account) in [(mint_x, user_x), (mint_y, user_y)] {
        let create_ix = associated_token::spl_associated_token_account::instruction::create_associated_token_account(
            &user,
            &user,
            &mint,
            &TOKEN_PROGRAM_ID,
        );
        svm.send_transaction(Transaction::new_signed_with_payer(
            &[create_ix],
            Some(&user),
            &[payer],
            svm.latest_blockhash(),
        ))
        .unwrap();

        let mint_ix = token::spl_token::instruction::mint_to(
            &TOKEN_PROGRAM_ID,
            &mint,
            &account,
            &user,
            &[],
            1_000_000_000,
        )
        .unwrap();
        svm.send_transaction(Transaction::new_signed_with_payer(
            &[mint_ix],
            Some(&user),
            &[payer],
            svm.latest_blockhash(),
        ))
        .unwrap();
    }

    let user_lp = associated_token::get_associated_token_address(&user, &mint_lp);

    Instruction::new_with_bytes(
        amm::id(),
        &amm::instruction::Deposit {
            amount: 100_000_000,
            max_x: 200_000_000,
            max_y: 200_000_000,
        }
        .data(),
        amm::accounts::Deposit {
            user,
            mint_x,
            mint_y,
            config,
            mint_lp,
            vault_x,
            vault_y,
            user_x,
            user_y,
            user_lp,
            token_program: TOKEN_PROGRAM_ID,
            associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
            system_program: SYSTEM_PROGRAM_ID,
        }
        .to_account_metas(None),
    )
}