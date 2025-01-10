use crate::decode::{AsyncRecordDecoder, RecordDecoder};
use crate::record_enum::RecordEnum;
use futures::stream::Stream;
use std::future::Future;
use std::io::Read;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::AsyncBufRead;

pub struct DecoderIterator<'a, R> {
    decoder: RecordDecoder<&'a mut R>,
}

impl<'a, R: Read> DecoderIterator<'a, R> {
    pub fn new(reader: &'a mut R) -> Self {
        Self {
            decoder: RecordDecoder::new(reader),
        }
    }
}

impl<'a, R: Read> Iterator for DecoderIterator<'a, R> {
    type Item = std::io::Result<RecordEnum>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.decoder.decode_ref() {
            Ok(Some(record_ref)) => match RecordEnum::from_ref(record_ref) {
                Ok(record) => Some(Ok(record)),
                Err(_) => Some(Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Failed to convert record reference to RecordEnum",
                ))),
            },
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

pub struct AsyncDecoderIterator<'a, R> {
    decoder: AsyncRecordDecoder<&'a mut R>,
}

impl<'a, R: AsyncBufRead + Unpin> AsyncDecoderIterator<'a, R> {
    pub fn new(reader: &'a mut R) -> Self {
        Self {
            decoder: AsyncRecordDecoder::new(reader),
        }
    }
}

impl<'a, R: AsyncBufRead + Unpin> Stream for AsyncDecoderIterator<'a, R> {
    type Item = std::io::Result<RecordEnum>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Poll for the next record asynchronously
        let fut = self.decoder.decode_ref();
        let mut fut = Box::pin(fut); // Pin the future

        match Future::poll(fut.as_mut(), cx) {
            Poll::Ready(Ok(Some(record_ref))) => {
                // If the record_ref is decoded successfully, convert it to RecordEnum
                match RecordEnum::from_ref(record_ref) {
                    Ok(record) => Poll::Ready(Some(Ok(record))),
                    Err(_) => Poll::Ready(Some(Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Failed to convert record reference to RecordEnum",
                    )))),
                }
            }
            Poll::Ready(Ok(None)) => Poll::Ready(None),
            Poll::Ready(Err(e)) => Poll::Ready(Some(Err(e))),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::*;
    use crate::encode::{CombinedEncoder, RecordEncoder};
    use crate::enums::Schema;
    use crate::metadata::Metadata;
    use crate::record_enum::RecordEnum;
    use crate::record_ref::*;
    use crate::records::BidAskPair;
    use crate::records::Mbp1Msg;
    use crate::records::OhlcvMsg;
    use crate::records::RecordHeader;
    use crate::symbols::SymbolMap;
    use futures::stream::StreamExt;
    use serial_test::serial;
    use std::io::Cursor;
    use std::path::PathBuf;

    async fn create_test_file() -> anyhow::Result<PathBuf> {
        // Metadata
        let mut symbol_map = SymbolMap::new();
        symbol_map.add_instrument("AAPL", 1);
        symbol_map.add_instrument("TSLA", 2);

        let metadata = Metadata::new(Schema::Mbp1, 1234567898765, 123456765432, symbol_map);

        // Record
        let msg1 = Mbp1Msg {
            hd: RecordHeader::new::<Mbp1Msg>(1, 1622471124),
            price: 12345676543,
            size: 1234543,
            action: 0,
            side: 0,
            depth: 0,
            flags: 0,
            ts_recv: 1231,
            ts_in_delta: 123432,
            sequence: 23432,
            discriminator: 0,
            levels: [BidAskPair {
                bid_px: 10000000,
                ask_px: 200000,
                bid_sz: 3000000,
                ask_sz: 400000000,
                bid_ct: 50000000,
                ask_ct: 60000000,
            }],
        };
        let msg2 = Mbp1Msg {
            hd: RecordHeader::new::<Mbp1Msg>(1, 1622471124),
            price: 12345676543,
            size: 1234543,
            action: 0,
            side: 0,
            depth: 0,
            flags: 0,
            ts_recv: 1231,
            ts_in_delta: 123432,
            sequence: 23432,
            discriminator: 0,
            levels: [BidAskPair {
                bid_px: 10000000,
                ask_px: 200000,
                bid_sz: 3000000,
                ask_sz: 400000000,
                bid_ct: 50000000,
                ask_ct: 60000000,
            }],
        };

        let record_ref1: RecordRef = (&msg1).into();
        let record_ref2: RecordRef = (&msg2).into();
        let records = &[record_ref1, record_ref2];

        let mut buffer = Vec::new();
        let mut encoder = CombinedEncoder::new(&mut buffer);
        encoder
            .encode(&metadata, records)
            .expect("Error on encoding");

        // Test
        let file = PathBuf::from("tests/test_decode_iter.bin");
        let _ = encoder.write_to_file(&file, false);

        Ok(file)
    }

    async fn delete_test_file(file: PathBuf) -> anyhow::Result<()> {
        // Cleanup
        if file.exists() {
            std::fs::remove_file(&file).expect("Failed to delete the test file.");
        }
        Ok(())
    }

    // -- Sync --
    #[tokio::test]
    #[serial]
    // #[ignore]
    async fn test_record_decoder_iter() -> anyhow::Result<()> {
        let file_path = create_test_file().await?;
        // let file_path = PathBuf::from("tests/test.bin");

        // Test
        let mut decoder =
            Decoder::<std::io::BufReader<std::fs::File>>::from_file(file_path.clone())?;
        let mut decode_iter = decoder.decode_iterator();

        let mut all_records = Vec::new();
        while let Some(record_result) = decode_iter.next() {
            match record_result {
                Ok(record) => match record {
                    RecordEnum::Mbp1(msg) => {
                        all_records.push(msg);
                    }
                    _ => unimplemented!(),
                },
                Err(e) => {
                    println!("{:?}", e);
                }
            }
        }

        // println!("{:?}", all_records);

        // Validate
        assert!(all_records.len() > 0);

        // Cleanup
        delete_test_file(file_path).await?;

        Ok(())
    }

    #[test]
    #[serial]
    // #[ignore]
    fn test_iter_decode() {
        // Setup
        let ohlcv_msg1 = OhlcvMsg {
            hd: RecordHeader::new::<OhlcvMsg>(1, 1622471124),
            open: 100,
            high: 200,
            low: 50,
            close: 150,
            volume: 1000,
        };

        let ohlcv_msg2 = OhlcvMsg {
            hd: RecordHeader::new::<OhlcvMsg>(2, 1622471125),
            open: 110,
            high: 210,
            low: 55,
            close: 155,
            volume: 1100,
        };

        // Encode
        let mut buffer = Vec::new();
        {
            let mut encoder = RecordEncoder::new(&mut buffer);
            let record_ref1: RecordRef = (&ohlcv_msg1).into();
            let record_ref2: RecordRef = (&ohlcv_msg2).into();
            encoder
                .encode_records(&[record_ref1, record_ref2])
                .expect("Encoding failed");
        }

        // Decode
        let cursor = Cursor::new(buffer);
        let mut decoder = RecordDecoder::new(cursor);
        let iter = decoder.decode_iterator();

        // Test
        let mut i = 0;
        for record in iter {
            match record {
                Ok(record) => {
                    // Process the record
                    if i == 0 {
                        assert_eq!(record, RecordEnum::Ohlcv(ohlcv_msg1.clone()));
                    } else {
                        assert_eq!(record, RecordEnum::Ohlcv(ohlcv_msg2.clone()));
                    }
                    i = i + 1;
                }
                Err(e) => {
                    eprintln!("Error processing record: {:?}", e);
                }
            }
        }
    }

    // -- Async --
    #[tokio::test]
    #[serial]
    // #[ignore]
    async fn test_record_decoder_iter_async() -> anyhow::Result<()> {
        let file_path = create_test_file().await?;
        // let file_path = PathBuf::from("tests/test.bin");

        // Test
        let mut decoder =
            <AsyncDecoder<tokio::io::BufReader<tokio::fs::File>>>::from_file(file_path.clone())
                .await?;
        let mut decode_iter = decoder.decode_iterator();

        let mut all_records = Vec::new();
        while let Some(record_result) = decode_iter.next().await {
            match record_result {
                Ok(record) => match record {
                    RecordEnum::Mbp1(msg) => {
                        all_records.push(msg);
                    }
                    _ => unimplemented!(),
                },
                Err(e) => {
                    println!("{:?}", e);
                }
            }
        }
        // println!("{:?}", all_records);

        // Validate
        assert!(all_records.len() > 0);

        // Cleanup
        delete_test_file(file_path).await?;

        Ok(())
    }

    #[tokio::test]
    #[serial]
    // #[ignore]
    async fn test_iter_decode_async() -> anyhow::Result<()> {
        // Setup
        let ohlcv_msg1 = OhlcvMsg {
            hd: RecordHeader::new::<OhlcvMsg>(1, 1622471124),
            open: 100,
            high: 200,
            low: 50,
            close: 150,
            volume: 1000,
        };

        let ohlcv_msg2 = OhlcvMsg {
            hd: RecordHeader::new::<OhlcvMsg>(2, 1622471125),
            open: 110,
            high: 210,
            low: 55,
            close: 155,
            volume: 1100,
        };

        // Encode
        let mut buffer = Vec::new();
        {
            let mut encoder = RecordEncoder::new(&mut buffer);
            let record_ref1: RecordRef = (&ohlcv_msg1).into();
            let record_ref2: RecordRef = (&ohlcv_msg2).into();
            encoder
                .encode_records(&[record_ref1, record_ref2])
                .expect("Encoding failed");
        }

        // Decode
        let cursor = Cursor::new(buffer);
        let mut decoder = AsyncRecordDecoder::new(cursor);
        let mut iter = decoder.decode_iterator();

        // Test
        let mut i = 0;
        while let Some(record) = iter.next().await {
            match record {
                Ok(record) => {
                    if i == 0 {
                        assert_eq!(record, RecordEnum::Ohlcv(ohlcv_msg1.clone()));
                    } else {
                        assert_eq!(record, RecordEnum::Ohlcv(ohlcv_msg2.clone()));
                    }
                    i = i + 1;
                }
                Err(e) => {
                    eprintln!("Error processing record: {:?}", e);
                }
            }
        }
        Ok(())
    }
}
