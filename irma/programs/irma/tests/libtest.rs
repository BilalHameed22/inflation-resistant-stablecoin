#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use anchor_lang::prelude::*;
    use anchor_lang::prelude::Pubkey;
    use anchor_lang::prelude::Clock;
    use anchor_lang::prelude::Sysvar;
    use anchor_lang::prelude::Signer;
    // use anchor_lang::prelude::Account;
    use anchor_lang::prelude::Program;
    use anchor_lang::context::Context;
    use anchor_lang::solana_program::sysvar::clock::ID as CLOCK_ID;
    use anchor_lang::system_program;
    use anchor_lang::Accounts;

    use openbook_v2::state::EventHeap; // {EventHeap, Market};
    use openbook_v2::typedefs::{EventHeapHeader, EventNode, AnyEvent, OracleConfig};
    use openbook_v2::ix_accounts::{ConsumeEvents, PlaceOrder};
    use openbook_v2::ID as OPENBOOKV2_ID;

    use irma::irma as money;
    use irma::pricing::{StateMap, StableState};
    use irma::IRMA_ID;
    use irma::pricing::MAX_BACKING_COUNT;
    use irma::{Init, Common, Maint};
    use irma::pricing::{init_pricing, set_mint_price, mint_irma, redeem_irma, list_reserves};
    use irma::State;



    #[test]
    fn test_crank() -> Result<()> {


        fn verify_event_heap_state<'c: 'info, 'info>(ctx: CpiContext<'_, '_ , 'c, 'info, ConsumeEvents<'info>>, operation: &str) -> Result<()> {
            let event_heap_account = &ctx.accounts.event_heap;
            let event_heap_data = event_heap_account.data.borrow();
            
            msg!("=== EventHeap verification after {} ===", operation);
            msg!("Account key: {:?}", event_heap_account.key);
            msg!("Account owner: {:?}", event_heap_account.owner);
            msg!("Data length: {}", event_heap_data.len());
            msg!("Expected size: {}", std::mem::size_of::<EventHeap>());
            
            if event_heap_data.len() >= std::mem::size_of::<EventHeapHeader>() {
                match EventHeap::try_from_slice(&event_heap_data) {
                    Ok(heap) => {
                        msg!("✅ EventHeap deserialization successful");
                        msg!("Header: free_head={}, used_head={}, count={}, seq_num={}", 
                            heap.header.free_head, heap.header.used_head, 
                            heap.header.count, heap.header.seq_num);
                        
                        // Check first few nodes
                        if heap.header.count > 0 && heap.nodes.len() > 0 {
                            msg!("First node: event_type={}, next={}, prev={}", 
                                heap.nodes[0].event.event_type,
                                heap.nodes[0].next,
                                heap.nodes[0].prev);
                        }
                    },
                    Err(e) => {
                        msg!("❌ EventHeap deserialization failed: {:?}", e);
                        // Log first few bytes for debugging
                        let preview_len = std::cmp::min(32, event_heap_data.len());
                        let preview: Vec<u8> = event_heap_data[..preview_len].to_vec();
                        msg!("First {} bytes: {:?}", preview_len, preview);
                    }
                }
            } else {
                msg!("❌ Data too small for EventHeap");
            }
            
            Ok(())
        }

        let crank_result: std::result::Result<(), Error> = money::crank(this_ctx);
        assert!(crank_result.is_ok());
        verify_event_heap_state(this_ctx, "consume_given_events")?;
        msg!("Crank executed successfully");

        msg!("Crank market completed successfully.");
        Ok(())
    }
}
