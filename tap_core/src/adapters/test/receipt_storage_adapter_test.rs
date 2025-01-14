// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod receipt_storage_adapter_unit_test {
    use std::collections::HashMap;
    use std::str::FromStr;
    use std::sync::Arc;

    use alloy_primitives::Address;
    use alloy_sol_types::{eip712_domain, Eip712Domain};
    use ethers::signers::coins_bip39::English;
    use ethers::signers::{LocalWallet, MnemonicBuilder};
    use rstest::*;
    use tokio::sync::RwLock;

    use crate::adapters::{
        receipt_storage_adapter::ReceiptStorageAdapter,
        receipt_storage_adapter_mock::ReceiptStorageAdapterMock,
    };
    use crate::{
        eip_712_signed_message::EIP712SignedMessage, tap_receipt::get_full_list_of_checks,
        tap_receipt::Receipt, tap_receipt::ReceivedReceipt,
    };

    #[fixture]
    fn domain_separator() -> Eip712Domain {
        eip712_domain! {
            name: "TAP",
            version: "1",
            chain_id: 1,
            verifying_contract: Address::from([0x11u8; 20]),
        }
    }

    #[rstest]
    #[tokio::test]
    async fn receipt_adapter_test(domain_separator: Eip712Domain) {
        let receipt_storage = Arc::new(RwLock::new(HashMap::new()));
        let mut receipt_adapter = ReceiptStorageAdapterMock::new(receipt_storage);

        let wallet: LocalWallet = MnemonicBuilder::<English>::default()
         .phrase("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about")
         .build()
         .unwrap();

        let allocation_id =
            Address::from_str("0xabababababababababababababababababababab").unwrap();

        // Create receipts
        let query_id = 10u64;
        let value = 100u128;
        let received_receipt = ReceivedReceipt::new(
            EIP712SignedMessage::new(
                &domain_separator,
                Receipt::new(allocation_id, value).unwrap(),
                &wallet,
            )
            .await
            .unwrap(),
            query_id,
            &get_full_list_of_checks(),
        );

        let receipt_store_result = receipt_adapter.store_receipt(received_receipt).await;
        assert!(receipt_store_result.is_ok());
        let receipt_id = receipt_store_result.unwrap();

        // Retreive receipt with id expected to be valid
        assert!(receipt_adapter
            .retrieve_receipt_by_id(receipt_id)
            .await
            .is_ok());
        // Retreive receipt with arbitrary id expected to be invalid
        assert!(receipt_adapter.retrieve_receipt_by_id(999).await.is_err());

        // Remove receipt with id expected to be valid
        assert!(receipt_adapter
            .remove_receipt_by_id(receipt_id)
            .await
            .is_ok());
        // Remove receipt with arbitrary id expected to be invalid
        assert!(receipt_adapter.remove_receipt_by_id(999).await.is_err());

        // Retreive receipt that was removed previously
        assert!(receipt_adapter
            .retrieve_receipt_by_id(receipt_id)
            .await
            .is_err());

        // Remove receipt that was removed previously
        assert!(receipt_adapter
            .remove_receipt_by_id(receipt_id)
            .await
            .is_err());
    }

    #[rstest]
    #[tokio::test]
    async fn multi_receipt_adapter_test(domain_separator: Eip712Domain) {
        let receipt_storage = Arc::new(RwLock::new(HashMap::new()));
        let mut receipt_adapter = ReceiptStorageAdapterMock::new(receipt_storage);

        let wallet: LocalWallet = MnemonicBuilder::<English>::default()
         .phrase("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about")
         .build()
         .unwrap();

        let allocation_id =
            Address::from_str("0xabababababababababababababababababababab").unwrap();

        // Create receipts
        let mut received_receipts = Vec::new();
        for (query_id, value) in (50..60).enumerate() {
            received_receipts.push(ReceivedReceipt::new(
                EIP712SignedMessage::new(
                    &domain_separator,
                    Receipt::new(allocation_id, value).unwrap(),
                    &wallet,
                )
                .await
                .unwrap(),
                query_id as u64,
                &get_full_list_of_checks(),
            ));
        }
        let mut receipt_ids = Vec::new();
        let mut receipt_timestamps = Vec::new();
        for received_receipt in received_receipts {
            receipt_ids.push(
                receipt_adapter
                    .store_receipt(received_receipt.clone())
                    .await
                    .unwrap(),
            );
            receipt_timestamps.push(received_receipt.signed_receipt.message.timestamp_ns)
        }

        // Retreive receipts with timestamp
        assert!(receipt_adapter
            .retrieve_receipts_by_timestamp(receipt_timestamps[0])
            .await
            .is_ok());
        assert!(!receipt_adapter
            .retrieve_receipts_by_timestamp(receipt_timestamps[0])
            .await
            .unwrap()
            .is_empty());

        // Retreive receipts before timestamp
        assert!(receipt_adapter
            .retrieve_receipts_upto_timestamp(receipt_timestamps[3])
            .await
            .is_ok());
        assert!(
            receipt_adapter
                .retrieve_receipts_upto_timestamp(receipt_timestamps[3])
                .await
                .unwrap()
                .len()
                >= 4
        );

        // Remove all receipts with one call
        assert!(receipt_adapter
            .remove_receipts_by_ids(receipt_ids.as_slice())
            .await
            .is_ok());
        // Removal should no longer be valid
        assert!(receipt_adapter
            .remove_receipts_by_ids(receipt_ids.as_slice())
            .await
            .is_err());
        // Retrieval should be invalid
        for receipt_id in receipt_ids {
            assert!(receipt_adapter
                .retrieve_receipt_by_id(receipt_id)
                .await
                .is_err());
        }
    }
}
