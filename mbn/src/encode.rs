use crate::metadata::Metadata;
use crate::record_ref::*;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::Path;
use tokio::io::{AsyncWrite, AsyncWriteExt};

pub struct CombinedEncoder<W> {
    writer: W,
}

impl<W: Write> CombinedEncoder<W> {
    pub fn new(writer: W) -> Self {
        CombinedEncoder { writer }
    }

    pub fn encode_metadata(&mut self, metadata: &Metadata) -> io::Result<()> {
        let mut metadata_encoder = MetadataEncoder::new(&mut self.writer);
        metadata_encoder.encode_metadata(metadata)
    }

    pub fn encode_record(&mut self, record: &RecordRef) -> io::Result<()> {
        let mut record_encoder = RecordEncoder::new(&mut self.writer);
        record_encoder.encode_record(record)
    }

    pub fn encode_records(&mut self, records: &[RecordRef]) -> io::Result<()> {
        let mut record_encoder = RecordEncoder::new(&mut self.writer);
        record_encoder.encode_records(records)
    }

    pub fn encode(&mut self, metadata: &Metadata, records: &[RecordRef]) -> io::Result<()> {
        self.encode_metadata(metadata)?;
        self.encode_records(records)?;
        Ok(())
    }

    pub fn write_to_file(&self, file_path: &Path, append: bool) -> io::Result<()>
    where
        W: AsRef<[u8]>,
    {
        let mut options = OpenOptions::new();
        options.create(true);

        if append {
            options.append(true);
        } else {
            options.write(true).truncate(true);
        }

        let mut file = options.open(file_path)?;

        file.write_all(self.writer.as_ref())?;
        file.flush()?;
        Ok(())
    }
}

pub struct MetadataEncoder<W> {
    writer: W,
    // buffer: Vec<u8>,
}

impl<W: Write> MetadataEncoder<W> {
    pub fn new(writer: W) -> Self {
        MetadataEncoder { writer }
    }

    pub fn encode_metadata(&mut self, metadata: &Metadata) -> io::Result<()> {
        let bytes = metadata.serialize();

        // Calculate and prepend the length
        let length: u16 = bytes.len() as u16;
        let mut buffer = Vec::with_capacity(length as usize + 2);

        // Add length as the first 2 bytes
        buffer.extend_from_slice(&length.to_le_bytes());
        buffer.extend_from_slice(&bytes);

        // Write the buffer to the writer
        self.writer.write_all(&buffer)?;
        self.writer.flush()?;
        Ok(())

        // self.buffer[..serialized.len()].copy_from_slice(&serialized);
        // self.writer.write_all(&self.buffer)?;
        // self.writer.flush()?;
        // Ok(())
    }
}

pub struct RecordEncoder<W> {
    writer: W,
}

impl<W: Write> RecordEncoder<W> {
    pub fn new(writer: W) -> Self {
        RecordEncoder { writer }
    }

    pub fn encode_record(&mut self, record: &RecordRef) -> io::Result<()> {
        let bytes = record.as_ref();
        self.writer.write_all(bytes)?;
        Ok(())
    }

    pub fn encode_records(&mut self, records: &[RecordRef]) -> io::Result<()> {
        for record in records {
            self.encode_record(record)?;
        }
        self.writer.flush()?;
        Ok(())
    }

    pub fn write_to_file(&self, file_path: &Path, append: bool) -> io::Result<()>
    where
        W: AsRef<[u8]>,
    {
        let mut options = OpenOptions::new();
        options.create(true);

        if append {
            options.append(true);
        } else {
            options.write(true).truncate(true);
        }

        let mut file = options.open(file_path)?;

        file.write_all(self.writer.as_ref())?;
        file.flush()?;
        Ok(())
    }
}

// -- Aysnc --

pub struct AsyncRecordEncoder<W> {
    writer: W,
}

impl<W> AsyncRecordEncoder<W>
where
    W: AsyncWrite + Unpin,
{
    pub fn new(writer: W) -> Self {
        AsyncRecordEncoder { writer }
    }

    pub async fn encode_record<'a>(&mut self, record: &'a RecordRef<'a>) -> tokio::io::Result<()> {
        let bytes = record.as_ref();
        self.writer.write_all(bytes).await?;
        Ok(())
    }

    pub async fn encode_records<'a>(
        &mut self,
        records: &'a [RecordRef<'a>],
    ) -> tokio::io::Result<()> {
        for record in records {
            self.encode_record(record).await?;
        }
        self.writer.flush().await?;
        Ok(())
    }
    pub async fn write_to_file(
        file_path: &Path,
        append: bool,
        buffer: &[u8],
    ) -> tokio::io::Result<()> {
        let mut options = tokio::fs::OpenOptions::new();
        options.create(true);

        if append {
            options.append(true);
        } else {
            options.write(true).truncate(true);
        }

        let mut file = options.open(file_path).await?;

        file.write_all(buffer).await?;
        file.flush().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serial_test::serial;

    use super::*;
    use crate::decode::AsyncDecoder;
    use crate::enums::Schema;
    use crate::record_enum::RecordEnum;
    use crate::records::BidAskPair;
    use crate::records::Mbp1Msg;
    use crate::records::OhlcvMsg;
    use crate::records::RecordHeader;
    use crate::symbols::SymbolMap;
    use std::io::Cursor;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_async_encode_record() -> anyhow::Result<()> {
        let ohlcv_msg = OhlcvMsg {
            hd: RecordHeader::new::<OhlcvMsg>(1, 1622471124),
            open: 100,
            high: 200,
            low: 50,
            close: 150,
            volume: 1000,
        };
        let record_ref: RecordRef = (&ohlcv_msg).into();

        // Test
        let mut buffer = Vec::new();
        let mut encoder = AsyncRecordEncoder::new(&mut buffer);
        encoder
            .encode_record(&record_ref)
            .await
            .expect("Encoding failed");

        // Validate
        let cursor = Cursor::new(buffer);
        let mut decoder = AsyncDecoder::new(cursor).await?;
        let record_ref = decoder.decode_ref().await?.unwrap();
        let decoded_record: &OhlcvMsg = record_ref.get().unwrap();
        assert_eq!(decoded_record, &ohlcv_msg);

        Ok(())
    }

    #[tokio::test]
    async fn test_async_encode_records() -> anyhow::Result<()> {
        let ohlcv_msg1 = OhlcvMsg {
            hd: RecordHeader::new::<OhlcvMsg>(1, 1622471124),
            open: 100000000000,
            high: 200000000000,
            low: 50000000000,
            close: 150000000000,
            volume: 1000,
        };

        let ohlcv_msg2 = OhlcvMsg {
            hd: RecordHeader::new::<OhlcvMsg>(2, 1622471125),
            open: 110000000000,
            high: 210000000000,
            low: 55000000000,
            close: 155000000000,
            volume: 1100,
        };

        let record_ref1: RecordRef = (&ohlcv_msg1).into();
        let record_ref2: RecordRef = (&ohlcv_msg2).into();

        // Test
        let mut buffer = Vec::new();
        let mut encoder = AsyncRecordEncoder::new(&mut buffer);
        encoder
            .encode_records(&[record_ref1, record_ref2])
            .await
            .expect("Encoding failed");
        // println!("{:?}", buffer);

        // Validate
        let cursor = Cursor::new(buffer);
        let mut decoder = AsyncDecoder::new(cursor).await?;
        let decoded_records = decoder.decode().await?;

        assert_eq!(decoded_records.len(), 2);
        assert_eq!(decoded_records[0], RecordEnum::Ohlcv(ohlcv_msg1));
        assert_eq!(decoded_records[1], RecordEnum::Ohlcv(ohlcv_msg2));

        Ok(())
    }

    #[tokio::test]
    async fn test_encode_record() -> anyhow::Result<()> {
        let ohlcv_msg = OhlcvMsg {
            hd: RecordHeader::new::<OhlcvMsg>(1, 1622471124),
            open: 100,
            high: 200,
            low: 50,
            close: 150,
            volume: 1000,
        };
        let record_ref: RecordRef = (&ohlcv_msg).into();

        // Test
        let mut buffer = Vec::new();
        let mut encoder = RecordEncoder::new(&mut buffer);
        encoder.encode_record(&record_ref).expect("Encoding failed");

        // Validate
        let cursor = Cursor::new(buffer);
        let mut decoder = AsyncDecoder::new(cursor).await?;
        let record_ref = decoder.decode_ref().await?.unwrap();
        let decoded_record: &OhlcvMsg = record_ref.get().unwrap();
        assert_eq!(decoded_record, &ohlcv_msg);

        Ok(())
    }

    #[tokio::test]
    async fn test_encode_decode_records() -> anyhow::Result<()> {
        let ohlcv_msg1 = OhlcvMsg {
            hd: RecordHeader::new::<OhlcvMsg>(1, 1622471124),
            open: 100000000000,
            high: 200000000000,
            low: 50000000000,
            close: 150000000000,
            volume: 1000,
        };

        let ohlcv_msg2 = OhlcvMsg {
            hd: RecordHeader::new::<OhlcvMsg>(2, 1622471125),
            open: 110000000000,
            high: 210000000000,
            low: 55000000000,
            close: 155000000000,
            volume: 1100,
        };

        let record_ref1: RecordRef = (&ohlcv_msg1).into();
        let record_ref2: RecordRef = (&ohlcv_msg2).into();

        // Test
        let mut buffer = Vec::new();
        let mut encoder = RecordEncoder::new(&mut buffer);
        encoder
            .encode_records(&[record_ref1, record_ref2])
            .expect("Encoding failed");
        // println!("{:?}", buffer);

        // Validate
        let cursor = Cursor::new(buffer);
        let mut decoder = AsyncDecoder::new(cursor).await?;
        let decoded_records = decoder.decode().await?;

        assert_eq!(decoded_records.len(), 2);
        assert_eq!(decoded_records[0], RecordEnum::Ohlcv(ohlcv_msg1));
        assert_eq!(decoded_records[1], RecordEnum::Ohlcv(ohlcv_msg2));

        Ok(())
    }

    #[tokio::test]
    async fn test_encode_metadata() -> anyhow::Result<()> {
        let mut symbol_map = SymbolMap::new();
        symbol_map.add_instrument("AAPL", 1);
        symbol_map.add_instrument("TSLA", 2);

        let metadata = Metadata::new(Schema::Ohlcv1S, 1234567898765, 123456765432, symbol_map);

        // Test
        let mut buffer = Vec::new();
        let mut encoder = MetadataEncoder::new(&mut buffer);
        encoder
            .encode_metadata(&metadata)
            .expect("Error metadata encoding.");

        // Validate
        let length_buffer: [u8; 2] = buffer[..2].try_into()?;
        let metadata_length = u16::from_le_bytes(length_buffer) as usize;
        let bytes = &buffer[2..2 + metadata_length];
        let decoded = Metadata::deserialize(&bytes)?;
        assert_eq!(decoded.schema, metadata.schema);
        assert_eq!(decoded.start, metadata.start);
        assert_eq!(decoded.end, metadata.end);
        assert_eq!(decoded.mappings, metadata.mappings);
        Ok(())
    }

    #[test]
    fn test_encode() {
        // Metadata
        let mut symbol_map = SymbolMap::new();
        symbol_map.add_instrument("AAPL", 1);
        symbol_map.add_instrument("TSLA", 2);

        let metadata = Metadata::new(Schema::Ohlcv1S, 1234567898765, 123456765432, symbol_map);

        // Record
        let ohlcv_msg1 = OhlcvMsg {
            hd: RecordHeader::new::<OhlcvMsg>(1, 1724287878000000000),
            open: 100000000000,
            high: 200000000000,
            low: 50000000000,
            close: 150000000000,
            volume: 1000000000000,
        };

        let ohlcv_msg2 = OhlcvMsg {
            hd: RecordHeader::new::<OhlcvMsg>(2, 1724289878000000000),
            open: 110000000000,
            high: 210000000000,
            low: 55000000000,
            close: 155000000000,
            volume: 1100000000000,
        };

        let record_ref1: RecordRef = (&ohlcv_msg1).into();
        let record_ref2: RecordRef = (&ohlcv_msg2).into();
        let records = &[record_ref1, record_ref2];

        // Test
        let mut buffer = Vec::new();
        let mut encoder = CombinedEncoder::new(&mut buffer);
        encoder
            .encode(&metadata, records)
            .expect("Error on encoding");

        // Validate
        assert!(buffer.len() > 0);
    }

    #[tokio::test]
    #[serial]
    async fn test_encode_to_file_w_metadata() -> anyhow::Result<()> {
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
        let file = PathBuf::from("tests/mbp_w_metadata.bin");
        let _ = encoder.write_to_file(&file, false);

        // Validate
        let mut decoder =
            <AsyncDecoder<tokio::io::BufReader<tokio::fs::File>>>::from_file(file.clone()).await?;
        let records = decoder.decode().await?;
        let expected = vec![
            RecordEnum::from_ref(record_ref1)?,
            RecordEnum::from_ref(record_ref2)?,
        ];

        assert!(expected == records);

        // Cleanup
        if file.exists() {
            std::fs::remove_file(&file).expect("Failed to delete the test file.");
        }
        Ok(())
    }

    #[tokio::test]
    #[serial]
    // #[ignore]
    async fn test_encode_to_file_wout_metadata() -> anyhow::Result<()> {
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

        let mut buffer = Vec::new();
        let mut encoder = RecordEncoder::new(&mut buffer);
        encoder
            .encode_records(&[record_ref1, record_ref2])
            .expect("Encoding failed");

        // Test
        let file = PathBuf::from("tests/mbp_wout_metadata.bin");
        let _ = encoder.write_to_file(&file, false);

        // Validate
        let mut decoder =
            <AsyncDecoder<tokio::io::BufReader<tokio::fs::File>>>::from_file(file.clone()).await?;
        let records = decoder.decode().await?;
        let expected = vec![
            RecordEnum::from_ref(record_ref1)?,
            RecordEnum::from_ref(record_ref2)?,
        ];

        assert!(expected == records);

        // Cleanup
        if file.exists() {
            std::fs::remove_file(&file).expect("Failed to delete the test file.");
        }
        Ok(())
    }
}
