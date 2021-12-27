use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{msg, pubkey::Pubkey};

use crate::{
    create::CreateAccounts,
    utils::{calculate_external_deposit, calculate_fee_from_amount},
};

// Hardcoded program version
pub const PROGRAM_VERSION: u8 = 2;
pub const STRM_TREASURY: &str = "Ht5G1RhkcKnpLVLMhqJc5aqZ4wYUEbxbtZwGCVbgU7DL"; //todo: update
pub const MAX_STRING_SIZE: usize = 200;
pub const STRM_FEE_DEFAULT_PERCENT: f32 = 0.25;

/// The struct containing instructions for initializing a stream
#[derive(BorshDeserialize, BorshSerialize, Clone, Debug)]
#[repr(C)]
pub struct CreateParams {
    /// Timestamp when the tokens start vesting
    pub start_time: u64,
    /// Timestamp when all tokens are fully vested
    pub end_time: u64, /* todo: move to metadata, calculate based on cliff, period,
                        * amount_per_period (not Create stream input params) */
    /// Deposited amount of tokens
    pub net_amount_deposited: u64,
    /// Time step (period) in seconds per which the vesting occurs
    pub period: u64,
    /// Amount released per period
    pub amount_per_period: u64,
    /// Vesting contract "cliff" timestamp
    pub cliff: u64,
    /// Amount unlocked at the "cliff" timestamp
    pub cliff_amount: u64,
    /// Whether or not a stream can be canceled by a sender
    pub cancelable_by_sender: bool,
    /// Whether or not a stream can be canceled by a recipient
    pub cancelable_by_recipient: bool,
    /// Whether or not a 3rd party can initiate withdraw in the name of recipient
    pub withdrawal_public: bool,
    /// Whether or not the sender can transfer the stream
    pub transferable_by_sender: bool,
    /// Whether or not the recipient can transfer the stream
    pub transferable_by_recipient: bool,
    /// Release rate of recurring payment
    pub release_rate: u64,
    /// The name of this stream
    pub stream_name: String,
}

/// TokenStreamData is the struct containing metadata for an SPL token stream.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
#[repr(C)]
pub struct Contract {
    /// Magic bytes
    pub magic: u64,
    /// Version of the program
    pub version: u8,
    /// Timestamp when stream was created
    pub created_at: u64,
    /// Amount of funds withdrawn
    pub amount_withdrawn: u64,
    /// Timestamp when stream was canceled (if canceled)
    pub canceled_at: u64,
    /// Timestamp at which stream can be safely canceled by a 3rd party
    /// (Stream is either fully vested or there isn't enough capital to
    /// keep it active)
    pub closable_at: u64, //TODO: remove, calculate end_date and use that as closable_at
    /// Timestamp of the last withdrawal
    pub last_withdrawn_at: u64,
    /// Pubkey of the stream initializer
    pub sender: Pubkey,
    /// Pubkey of the stream initializer's token account
    pub sender_tokens: Pubkey,
    /// Pubkey of the stream recipient
    pub recipient: Pubkey,
    /// Pubkey of the stream recipient's token account
    pub recipient_tokens: Pubkey,
    /// Pubkey of the token mint
    pub mint: Pubkey,
    /// Escrow account holding the locked tokens for recipient
    pub escrow_tokens: Pubkey,
    /// Streamflow treasury authority
    pub streamflow_treasury: Pubkey,
    /// Escrow account holding the locked tokens for Streamflow (fee account)
    pub streamflow_treasury_tokens: Pubkey,
    /// The total fee amount for streamflow
    pub streamflow_fee_total: u64,
    /// The withdrawn fee amount for streamflow
    pub streamflow_fee_withdrawn: u64,
    /// Fee percentage for Streamflow
    pub streamflow_fee_percent: f32,
    /// Streamflow partner authority
    pub partner: Pubkey,
    /// Escrow account holding the locked tokens for Streamflow partner (fee account)
    pub partner_tokens: Pubkey,
    /// The total fee amount for the partner
    pub partner_fee_total: u64,
    /// The withdrawn fee amount for the partner
    pub partner_fee_withdrawn: u64,
    /// Fee percentage for partner
    pub partner_fee_percent: f32,
    /// The stream instruction
    pub ix: CreateParams,
}

impl Contract {
    /// Initialize a new `TokenStreamData` struct.
    pub fn new(
        now: u64,
        acc: CreateAccounts,
        ix: CreateParams,
        partner_fee_total: u64,
        partner_fee_percent: f32,
        streamflow_fee_total: u64,
        streamflow_fee_percent: f32,
    ) -> Self {
        // TODO: calculate end_time based on other parameters (incl. net_amount_deposited)
        Self {
            magic: 0,
            version: PROGRAM_VERSION,
            created_at: now,
            amount_withdrawn: 0,
            canceled_at: 0,
            closable_at: ix.end_time,
            last_withdrawn_at: 0,
            sender: *acc.sender.key,
            sender_tokens: *acc.sender_tokens.key,
            recipient: *acc.recipient.key,
            recipient_tokens: *acc.recipient_tokens.key,
            mint: *acc.mint.key,
            escrow_tokens: *acc.escrow_tokens.key,
            streamflow_treasury: *acc.streamflow_treasury.key,
            streamflow_treasury_tokens: *acc.streamflow_treasury_tokens.key,
            streamflow_fee_total,
            streamflow_fee_withdrawn: 0,
            streamflow_fee_percent,
            partner: *acc.partner.key,
            partner_tokens: *acc.partner_tokens.key,
            partner_fee_total,
            partner_fee_withdrawn: 0,
            partner_fee_percent,
            ix,
        }
    }

    /// Calculate timestamp when stream is closable
    /// end_time when deposit == total else time when funds run out
    pub fn closable(&self) -> u64 {
        let cliff_time = if self.ix.cliff > 0 { self.ix.cliff } else { self.ix.start_time };

        let cliff_amount = if self.ix.cliff_amount > 0 { self.ix.cliff_amount } else { 0 };
        // Deposit smaller then cliff amount, cancelable at cliff
        if self.ix.net_amount_deposited < cliff_amount {
            return cliff_time
        }
        // Nr of seconds after the cliff
        let seconds_nr = self.ix.end_time - cliff_time;

        let amount_per_second = if self.ix.release_rate > 0 {
            self.ix.release_rate / self.ix.period
        } else {
            // stream per second
            ((self.ix.net_amount_deposited - cliff_amount) / seconds_nr) as u64
        };
        // Seconds till account runs out of available funds, +1 as ceil (integer)
        let seconds_left = ((self.ix.net_amount_deposited - cliff_amount) / amount_per_second) + 1;

        msg!(
            "Release {}, Period {}, seconds left {}",
            self.ix.release_rate,
            self.ix.period,
            seconds_left
        );
        // closable_at time, ignore end_time when recurring
        if cliff_time + seconds_left > self.ix.end_time && self.ix.release_rate == 0 {
            self.ix.end_time
        } else {
            cliff_time + seconds_left
        }
    }

    pub fn sync_balance(&mut self, balance: u64) {
        let external_deposit = calculate_external_deposit(
            balance,
            self.ix.net_amount_deposited,
            self.amount_withdrawn,
        );

        if external_deposit > 0 {
            self.deposit(external_deposit);
        }
    }

    pub fn deposit(&mut self, amount: u64) {
        let partner_fee_addition = calculate_fee_from_amount(amount, self.partner_fee_percent);
        let strm_fee_addition = calculate_fee_from_amount(amount, self.partner_fee_percent);
        self.ix.net_amount_deposited += (amount - partner_fee_addition - strm_fee_addition);
        self.partner_fee_total += partner_fee_addition;
        self.streamflow_fee_total += strm_fee_addition;
    }
}
