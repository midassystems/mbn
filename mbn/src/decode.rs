use crate::decode_iterator::{AsyncDecoderIterator, DecoderIterator};
use crate::metadata::Metadata;
use crate::record_enum::RecordEnum;
use crate::record_ref::*;
use crate::records::RecordHeader;
use std::io::{BufReader, Read};
use std::mem;
use std::path::Path;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt};

pub struct Decoder<R> {
    pub metadata: Option<Metadata>,
    decoder: RecordDecoder<R>,
}

impl<R: Read> Decoder<R> {
    pub fn new(mut reader: R) -> std::io::Result<Self> {
        let metadata = MetadataDecoder::new(&mut reader).decode()?;

        Ok(Self {
            metadata,
            decoder: RecordDecoder::new(reader),
        })
    }
    pub fn metadata(&mut self) -> Option<Metadata> {
        self.metadata.clone()
    }

    pub fn decode(&mut self) -> crate::error::Result<Vec<RecordEnum>> {
        Ok(self.decoder.decode_to_owned()?)
    }

    pub fn decode_ref(&mut self) -> std::io::Result<Option<RecordRef>> {
        Ok(self.decoder.decode_ref()?)
    }

    pub fn decode_iterator(&mut self) -> DecoderIterator<R> {
        self.decoder.decode_iterator()
    }

    /// Accepts PathBuf, Path and str for file_path
    pub fn from_file<P: AsRef<Path>>(
        file_path: P,
    ) -> std::io::Result<Decoder<BufReader<std::fs::File>>> {
        let file = std::fs::File::open(file_path.as_ref())?;
        let buffered_reader = BufReader::new(file);

        Ok(Decoder::new(buffered_reader)?)
    }
}

pub struct MetadataDecoder<R> {
    reader: R,
    // read_buffer: Vec<u8>,
}

impl<R: Read> MetadataDecoder<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    /// Decodes metadata from the reader.
    pub fn decode(&mut self) -> std::io::Result<Option<Metadata>> {
        // Read the length field (first 2 bytes)
        let mut length_buffer = [0u8; 2];
        self.reader.read_exact(&mut length_buffer)?;

        // Decode the metadata length
        let metadata_length = u16::from_le_bytes(length_buffer) as usize;

        // Step 2: Read the metadata payload
        let mut read_buffer = vec![0u8; metadata_length];
        self.reader.read_exact(&mut read_buffer)?;

        // Step 3: Deserialize the metadata
        match Metadata::deserialize(&read_buffer) {
            Ok(metadata) => Ok(Some(metadata)),
            Err(_) => Ok(None), // Return None on deserialization failure
        }
    }
    // pub fn decode(&mut self) -> std::io::Result<Option<Metadata>> {
    //     // Try to read the buffer for metadata
    //     match self.reader.read_exact(&mut self.read_buffer) {
    //         Ok(_) => {
    //             // Attempt to deserialize the buffer if data is present
    //             match Metadata::deserialize(&self.read_buffer) {
    //                 Ok(metadata) => Ok(Some(metadata)),
    //                 Err(_) => {
    //                     // If deserialization fails, return None
    //                     Ok(None)
    //                 }
    //             }
    //         }
    //         Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
    //             // If we reach EOF without finding metadata, return None
    //             Ok(None)
    //         }
    //         Err(e) => Err(e),
    //     }
    // }
}

pub struct RecordDecoder<R> {
    reader: R,
    read_buffer: Vec<u8>,
}

impl<R> RecordDecoder<R>
where
    R: Read,
{
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            read_buffer: vec![0],
        }
    }

    pub fn decode_to_owned(&mut self) -> crate::error::Result<Vec<RecordEnum>> {
        let mut records = Vec::new();
        while let Some(record_ref) = self.decode_ref()? {
            let record = RecordEnum::from_ref(record_ref)?;
            records.push(record);
        }
        Ok(records)
    }

    pub fn decode_iterator(&mut self) -> DecoderIterator<R> {
        DecoderIterator::new(&mut self.reader)
    }

    pub fn decode_ref(&mut self) -> std::io::Result<Option<RecordRef>> {
        if let Err(err) = self.reader.read_exact(&mut self.read_buffer[..1]) {
            if err.kind() == std::io::ErrorKind::UnexpectedEof {
                return Ok(None);
            } else {
                return Err(std::io::Error::new(
                    err.kind(),
                    format!("decoding record reference: {}", err),
                ));
            }
        }
        let length = self.read_buffer[0] as usize * RecordHeader::LENGTH_MULTIPLIER;
        if length < mem::size_of::<RecordHeader>() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid record with length {} shorter than header", length),
            ));
        }
        if length > self.read_buffer.len() {
            self.read_buffer.resize(length, 0);
        }
        if let Err(err) = self.reader.read_exact(&mut self.read_buffer[1..length]) {
            if err.kind() == std::io::ErrorKind::UnexpectedEof {
                return Ok(None);
            } else {
                return Err(std::io::Error::new(
                    err.kind(),
                    format!("decoding record reference: {}", err),
                ));
            }
        }
        // Safety: `read_buffer` is resized to contain at least `length` bytes.
        Ok(Some(unsafe { RecordRef::new(&self.read_buffer) }))
    }

    pub fn from_file(file_path: &Path) -> std::io::Result<RecordDecoder<BufReader<std::fs::File>>> {
        let file = std::fs::File::open(file_path)?;
        let buffered_reader = BufReader::new(file);

        Ok(RecordDecoder::new(buffered_reader))
    }
}

// -- Async --
pub struct AsyncDecoder<R> {
    pub metadata: Option<Metadata>,
    decoder: AsyncRecordDecoder<R>,
}

impl<R: AsyncBufRead + Unpin> AsyncDecoder<R> {
    pub async fn new(mut reader: R) -> tokio::io::Result<Self> {
        let metadata = AsyncMetadataDecoder::new(&mut reader).decode().await?;

        Ok(Self {
            metadata,
            decoder: AsyncRecordDecoder::new(reader),
        })
    }

    pub fn metadata(&mut self) -> Option<Metadata> {
        self.metadata.clone()
    }

    pub async fn decode(&mut self) -> crate::error::Result<Vec<RecordEnum>> {
        Ok(self.decoder.decode_to_owned().await?)
    }

    pub async fn decode_ref(&mut self) -> tokio::io::Result<Option<RecordRef>> {
        Ok(self.decoder.decode_ref().await?)
    }

    pub fn decode_iterator(&mut self) -> AsyncDecoderIterator<R> {
        self.decoder.decode_iterator()
    }

    /// Accepts PathBuf, Path and str for file_path
    pub async fn from_file<P: AsRef<Path>>(
        file_path: P,
    ) -> tokio::io::Result<AsyncDecoder<tokio::io::BufReader<tokio::fs::File>>> {
        let file = tokio::fs::File::open(file_path.as_ref()).await?;
        let buffered_reader = tokio::io::BufReader::new(file);

        AsyncDecoder::new(buffered_reader).await
    }
}

pub struct AsyncMetadataDecoder<R> {
    reader: R,
    // read_buffer: Vec<u8>,
}

impl<R: AsyncBufRead + Unpin> AsyncMetadataDecoder<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            // read_buffer: vec![0; METADATA_LENGTH], // Initialize buffer with fixed size
        }
    }

    pub async fn decode(&mut self) -> tokio::io::Result<Option<Metadata>> {
        // Peek into the buffer without consuming bytes
        let buffer = self.reader.fill_buf().await?;

        // Ensure we have enough bytes for metadata
        if buffer.len() < 2 {
            return Ok(None); // Not enough data, return None (no metadata)
        }

        // Decode the metadata length
        let metadata_length = u16::from_le_bytes([buffer[0], buffer[1]]) as usize;

        // Attempt to deserialize the peeked bytes
        if buffer.len() < 2 + metadata_length {
            return Ok(None); // Not enough data yet
        }

        // Deserialize the metadata
        match Metadata::deserialize(&buffer[2..2 + metadata_length]) {
            Ok(metadata) => {
                // Consume the bytes used for metadata
                self.reader.consume(2 + metadata_length);
                Ok(Some(metadata))
            }
            Err(_) => Ok(None),
        }
    }
}

pub struct AsyncRecordDecoder<R> {
    reader: R,
    read_buffer: Vec<u8>,
}

impl<R> AsyncRecordDecoder<R>
where
    R: AsyncBufRead + AsyncReadExt + Unpin,
{
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            read_buffer: vec![0],
        }
    }

    pub async fn decode_to_owned(&mut self) -> crate::error::Result<Vec<RecordEnum>> {
        let mut records = Vec::new();
        while let Some(record_ref) = self.decode_ref().await? {
            let record = RecordEnum::from_ref(record_ref)?;
            records.push(record);
        }

        Ok(records)
    }

    pub fn decode_iterator(&mut self) -> AsyncDecoderIterator<R> {
        AsyncDecoderIterator::new(&mut self.reader)
    }

    pub async fn decode_ref(&mut self) -> tokio::io::Result<Option<RecordRef>> {
        if let Err(err) = self.reader.read_exact(&mut self.read_buffer[..1]).await {
            if err.kind() == tokio::io::ErrorKind::UnexpectedEof {
                return Ok(None);
            } else {
                return Err(tokio::io::Error::new(
                    err.kind(),
                    format!("decoding record reference: {}", err),
                ));
            }
        }
        let length = self.read_buffer[0] as usize * RecordHeader::LENGTH_MULTIPLIER;
        if length < mem::size_of::<RecordHeader>() {
            return Err(tokio::io::Error::new(
                tokio::io::ErrorKind::InvalidData,
                format!("invalid record with length {} shorter than header", length),
            ));
        }
        if length > self.read_buffer.len() {
            self.read_buffer.resize(length, 0);
        }
        if let Err(err) = self
            .reader
            .read_exact(&mut self.read_buffer[1..length])
            .await
        {
            if err.kind() == tokio::io::ErrorKind::UnexpectedEof {
                return Ok(None);
            } else {
                return Err(tokio::io::Error::new(
                    err.kind(),
                    format!("decoding record reference: {}", err),
                ));
            }
        }
        // Safety: `read_buffer` is resized to contain at least `length` bytes.
        Ok(Some(unsafe { RecordRef::new(&self.read_buffer) }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encode::MetadataEncoder;
    use crate::encode::{CombinedEncoder, RecordEncoder};
    use crate::enums::{RType, Schema};
    use crate::record_enum::RecordEnum;
    use crate::records::BidAskPair;
    use crate::records::Mbp1Msg;
    use crate::records::{as_u8_slice, OhlcvMsg};
    use crate::symbols::SymbolMap;
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
        let file = PathBuf::from("tests/test_decode.bin");
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
    // Decoder
    #[test]
    #[serial]
    // #[ignore]
    fn test_decode() -> anyhow::Result<()> {
        // Metadata
        let mut symbol_map = SymbolMap::new();
        symbol_map.add_instrument("AAPL", 1);
        symbol_map.add_instrument("TSLA", 2);

        let metadata = Metadata::new(Schema::Ohlcv1S, 1234567898765, 123456765432, symbol_map);

        // Record
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

        let record_ref1: RecordRef = (&ohlcv_msg1).into();
        let record_ref2: RecordRef = (&ohlcv_msg2).into();
        let records = &[record_ref1, record_ref2];

        let mut buffer = Vec::new();
        let mut encoder = CombinedEncoder::new(&mut buffer);
        encoder
            .encode(&metadata, records)
            .expect("Error on encoding");

        // Test
        let cursor = Cursor::new(buffer);
        let mut decoder = Decoder::new(cursor)?;
        let decoded = decoder.decode().expect("Error decoding metadata.");

        // Validate
        assert_eq!(
            decoded,
            [RecordEnum::Ohlcv(ohlcv_msg1), RecordEnum::Ohlcv(ohlcv_msg2)]
        );
        Ok(())
    }

    #[tokio::test]
    #[serial]
    // #[ignore]
    async fn test_combinder_decoder_from_file() -> anyhow::Result<()> {
        let file_path = create_test_file().await?;

        // let file_path = PathBuf::from("tests/test.bin");

        // Test
        let mut decoder =
            Decoder::<std::io::BufReader<std::fs::File>>::from_file(file_path.clone())?;
        let records = decoder.decode()?;
        // println!("{:?}", records);

        // Validate
        assert!(records.len() > 0);

        // Cleanup
        delete_test_file(file_path).await?;

        Ok(())
    }

    // MetadataDecoder
    #[test]
    #[serial]
    // #[ignore]
    fn test_decode_metadata() {
        let mut symbol_map = SymbolMap::new();
        symbol_map.add_instrument("AAPL", 1);
        symbol_map.add_instrument("TSLA", 2);

        let metadata = Metadata::new(Schema::Ohlcv1S, 1234567898765, 123456765432, symbol_map);

        let mut buffer = Vec::new();
        let mut encoder = MetadataEncoder::new(&mut buffer);
        encoder
            .encode_metadata(&metadata)
            .expect("Error metadata encoding.");

        // Test
        let cursor = Cursor::new(buffer);
        let mut decoder = MetadataDecoder::new(cursor);
        let decoded = decoder.decode().expect("Error decoding metadata.").unwrap();

        // Validate
        assert_eq!(decoded.schema, metadata.schema);
        assert_eq!(decoded.start, metadata.start);
        assert_eq!(decoded.end, metadata.end);
        assert_eq!(decoded.mappings, metadata.mappings);
    }

    // RecordDecoder
    #[test]
    #[serial]
    // #[ignore]
    fn test_decode_record() {
        // Create an OhlcvMsg record
        let ohlcv_msg = OhlcvMsg {
            hd: RecordHeader::new::<OhlcvMsg>(1, 1622471124),
            open: 100,
            high: 200,
            low: 50,
            close: 150,
            volume: 1000,
        };

        // Convert the record to a byte slice
        let data = unsafe { as_u8_slice(&ohlcv_msg) };

        // Create a cursor from the byte slice
        let cursor = Cursor::new(data.to_vec());

        // Initialize the RecordDecoder with the cursor
        let mut decoder = RecordDecoder::new(cursor);

        // Decode the record
        let result = decoder.decode_to_owned().expect("Error decoding.");

        // Verify the result
        if let Some(RecordEnum::Ohlcv(msg)) = result.get(0) {
            assert_eq!(msg.open, 100);
            assert_eq!(msg.high, 200);
            assert_eq!(msg.low, 50);
            assert_eq!(msg.close, 150);
            assert_eq!(msg.volume, 1000);
        } else {
            panic!("Failed to decode record");
        }
    }

    #[test]
    #[serial]
    // #[ignore]
    fn test_decode_record_ref() {
        // Create an OhlcvMsg record
        let ohlcv_msg = OhlcvMsg {
            hd: RecordHeader::new::<OhlcvMsg>(1, 1622471124),
            open: 100,
            high: 200,
            low: 50,
            close: 150,
            volume: 1000,
        };

        // Convert the record to a byte slice
        let data = unsafe { as_u8_slice(&ohlcv_msg) };

        // Create a cursor from the byte slice
        let cursor = Cursor::new(data.to_vec());

        // Initialize the RecordDecoder with the cursor
        let mut decoder = RecordDecoder::new(cursor);

        // Decode the record reference
        let result = decoder.decode_ref();

        // Verify the result
        assert!(result.is_ok());
        if let Some(record_ref) = result.unwrap() {
            let header = record_ref.header();
            assert_eq!(header.instrument_id, 1);
            assert_eq!(header.ts_event, 1622471124);
            assert_eq!(header.rtype, RType::Ohlcv as u8);
        } else {
            panic!("Failed to decode record reference");
        }
    }

    #[test]
    #[serial]
    // #[ignore]
    fn test_encode_decode_records() {
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

        let mut buffer = Vec::new();
        {
            let mut encoder = RecordEncoder::new(&mut buffer);
            let record_ref1: RecordRef = (&ohlcv_msg1).into();
            let record_ref2: RecordRef = (&ohlcv_msg2).into();
            encoder
                .encode_records(&[record_ref1, record_ref2])
                .expect("Encoding failed");
        }

        let cursor = Cursor::new(buffer);
        let mut decoder = RecordDecoder::new(cursor);

        let decoded_records: Vec<RecordEnum> = decoder.decode_to_owned().expect("Decoding failed");

        assert_eq!(decoded_records.len(), 2);
        assert_eq!(decoded_records[0], RecordEnum::Ohlcv(ohlcv_msg1));
        assert_eq!(decoded_records[1], RecordEnum::Ohlcv(ohlcv_msg2));
    }

    // -- Async --
    // Decoder
    #[tokio::test]
    #[serial]
    // #[ignore]
    async fn test_decode_async() -> anyhow::Result<()> {
        // Metadata
        let mut symbol_map = SymbolMap::new();
        symbol_map.add_instrument("AAPL", 1);
        symbol_map.add_instrument("TSLA", 2);

        let metadata = Metadata::new(Schema::Ohlcv1S, 1234567898765, 123456765432, symbol_map);

        // Record
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

        let record_ref1: RecordRef = (&ohlcv_msg1).into();
        let record_ref2: RecordRef = (&ohlcv_msg2).into();
        let records = &[record_ref1, record_ref2];

        let mut buffer = Vec::new();
        let mut encoder = CombinedEncoder::new(&mut buffer);
        encoder
            .encode(&metadata, records)
            .expect("Error on encoding");

        // Test
        let cursor = Cursor::new(buffer);
        let mut decoder = AsyncDecoder::new(cursor).await?;
        let decoded = decoder.decode().await?;

        // Validate
        assert_eq!(
            decoded,
            [RecordEnum::Ohlcv(ohlcv_msg1), RecordEnum::Ohlcv(ohlcv_msg2)]
        );
        Ok(())
    }

    #[tokio::test]
    #[serial]
    // #[ignore]
    async fn test_combinder_decoder_from_file_async() -> anyhow::Result<()> {
        let file_path = create_test_file().await?;

        // Test
        let mut decoder =
            <AsyncDecoder<tokio::io::BufReader<tokio::fs::File>>>::from_file(file_path.clone())
                .await?;
        let records = decoder.decode().await?;

        // Validate
        assert!(records.len() > 0);

        // Cleanup
        delete_test_file(file_path).await?;

        Ok(())
    }

    // MetadataDecoder
    #[tokio::test]
    #[serial]
    // #[ignore]
    async fn test_decode_metadata_async() -> anyhow::Result<()> {
        let mut symbol_map = SymbolMap::new();
        symbol_map.add_instrument("AAPL", 1);
        symbol_map.add_instrument("TSLA", 2);

        let metadata = Metadata::new(Schema::Ohlcv1S, 1234567898765, 123456765432, symbol_map);

        let mut buffer = Vec::new();
        let mut encoder = MetadataEncoder::new(&mut buffer);
        encoder
            .encode_metadata(&metadata)
            .expect("Error metadata encoding.");

        // Test
        let cursor = Cursor::new(buffer);
        let mut decoder = AsyncMetadataDecoder::new(cursor);
        let decoded = decoder.decode().await?.unwrap();

        // Validate
        assert_eq!(decoded.schema, metadata.schema);
        assert_eq!(decoded.start, metadata.start);
        assert_eq!(decoded.end, metadata.end);
        assert_eq!(decoded.mappings, metadata.mappings);

        Ok(())
    }

    // RecordDecoder
    #[tokio::test]
    #[serial]
    // #[ignore]
    async fn test_decode_record_async() -> anyhow::Result<()> {
        // Create an OhlcvMsg record
        let ohlcv_msg = OhlcvMsg {
            hd: RecordHeader::new::<OhlcvMsg>(1, 1622471124),
            open: 100,
            high: 200,
            low: 50,
            close: 150,
            volume: 1000,
        };

        // Convert the record to a byte slice
        let data = unsafe { as_u8_slice(&ohlcv_msg) };

        // Create a cursor from the byte slice
        let cursor = Cursor::new(data.to_vec());

        // Initialize the RecordDecoder with the cursor
        let mut decoder = AsyncRecordDecoder::new(cursor);

        // Decode the record
        let result = decoder.decode_to_owned().await?;

        // Verify the result
        if let Some(RecordEnum::Ohlcv(msg)) = result.get(0) {
            assert_eq!(msg.open, 100);
            assert_eq!(msg.high, 200);
            assert_eq!(msg.low, 50);
            assert_eq!(msg.close, 150);
            assert_eq!(msg.volume, 1000);
        } else {
            panic!("Failed to decode record");
        }
        Ok(())
    }

    #[tokio::test]
    #[serial]
    // #[ignore]
    async fn test_decode_record_ref_async() -> anyhow::Result<()> {
        // Create an OhlcvMsg record
        let ohlcv_msg = OhlcvMsg {
            hd: RecordHeader::new::<OhlcvMsg>(1, 1622471124),
            open: 100,
            high: 200,
            low: 50,
            close: 150,
            volume: 1000,
        };

        // Convert the record to a byte slice
        let data = unsafe { as_u8_slice(&ohlcv_msg) };

        // Create a cursor from the byte slice
        let cursor = Cursor::new(data.to_vec());

        // Initialize the RecordDecoder with the cursor
        let mut decoder = AsyncRecordDecoder::new(cursor);

        // Decode the record reference
        let record_ref = decoder.decode_ref().await?.unwrap();

        // Verify the result
        let header = record_ref.header();
        assert_eq!(header.instrument_id, 1);
        assert_eq!(header.ts_event, 1622471124);
        assert_eq!(header.rtype, RType::Ohlcv as u8);

        Ok(())
    }

    #[tokio::test]
    #[serial]
    // #[ignore]
    async fn test_encode_decode_records_async() -> anyhow::Result<()> {
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

        let mut buffer = Vec::new();
        {
            let mut encoder = RecordEncoder::new(&mut buffer);
            let record_ref1: RecordRef = (&ohlcv_msg1).into();
            let record_ref2: RecordRef = (&ohlcv_msg2).into();
            encoder
                .encode_records(&[record_ref1, record_ref2])
                .expect("Encoding failed");
        }

        let cursor = Cursor::new(buffer);
        let mut decoder = AsyncRecordDecoder::new(cursor);

        let decoded_records: Vec<RecordEnum> = decoder.decode_to_owned().await?;

        assert_eq!(decoded_records.len(), 2);
        assert_eq!(decoded_records[0], RecordEnum::Ohlcv(ohlcv_msg1));
        assert_eq!(decoded_records[1], RecordEnum::Ohlcv(ohlcv_msg2));

        Ok(())
    }
}
