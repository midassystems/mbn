use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::collections::HashMap;
use std::io;
use time::OffsetDateTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Vendors {
    Databento,
    Yfinance,
}

impl TryFrom<&str> for Vendors {
    type Error = crate::Error;

    fn try_from(s: &str) -> crate::Result<Vendors> {
        match s.to_lowercase().as_str() {
            "databento" => Ok(Vendors::Databento),
            "yfinance" => Ok(Vendors::Yfinance),
            _ => Err(crate::Error::CustomError(
                "Invalid vendor name.".to_string(),
            )),
        }
    }
}

impl Into<String> for Vendors {
    fn into(self) -> String {
        match self {
            Vendors::Databento => return "databento".to_string(),
            Vendors::Yfinance => return "yfinance".to_string(),
        }
    }
}

#[cfg(feature = "python")]
use pyo3::pyclass;

/// Struct representing a financial instrument.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Instrument {
    /// Midas unique instrument id number.
    pub instrument_id: Option<u32>,
    /// Instrument ticker.
    pub ticker: String,
    /// Instrument name e.g. Apple Inc.
    pub name: String,
    /// Vendor Name
    pub vendor: String,
    // Vendor Specific
    pub stype: Option<String>,
    // Vendor specific
    pub dataset: Option<String>,
    /// Last date available in database
    pub last_available: u64,
    /// first date available in database
    pub first_available: u64,
    /// Active status
    pub active: bool,
}

impl Instrument {
    pub fn new(
        instrument_id: Option<u32>,
        ticker: &str,
        name: &str,
        vendor: Vendors,
        stype: Option<String>,
        dataset: Option<String>,
        last_available: u64,
        first_available: u64,
        active: bool,
    ) -> Self {
        Self {
            instrument_id,
            ticker: ticker.to_string(),
            name: name.to_string(),
            vendor: vendor.into(),
            stype,
            dataset,
            last_available,
            first_available,
            active,
        }
    }

    /// Generic function to convert UNIX nanoseconds to OffsetDateTime
    fn timestamp_to_datetime(&self, timestamp: u64) -> Result<OffsetDateTime> {
        OffsetDateTime::from_unix_timestamp_nanos(timestamp as i128)
            .map_err(|_| Error::DateError("Error: Invalid UNIX nanosecond timestamp".to_string()))
    }

    /// Retrieve `first_available` as OffsetDateTime
    pub fn first_available_datetime(&self) -> Result<OffsetDateTime> {
        self.timestamp_to_datetime(self.first_available)
    }

    /// Retrieve `last_available` as OffsetDateTime
    pub fn last_available_datetime(&self) -> Result<OffsetDateTime> {
        self.timestamp_to_datetime(self.last_available)
    }
}

/// Struct created by Midas server to map instrument ids to tickers.
#[cfg_attr(feature = "python", pyclass(get_all, set_all, dict, module = "mbn"))]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SymbolMap {
    pub map: HashMap<u32, String>,
}

impl SymbolMap {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn add_instrument(&mut self, ticker: &str, id: u32) {
        self.map.insert(id, ticker.to_string());
    }

    pub fn get_instrument_ticker(&self, id: u32) -> Option<String> {
        self.map.get(&id).cloned()
    }

    /// Merges another SymbolMap into this one.
    pub fn merge(&mut self, other: &SymbolMap) {
        self.map.extend(other.map.clone());
    }

    /// Binary encodes struct for response, shouldn't be used directly.
    pub fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        let map_len = self.map.len() as u32;
        bytes.extend_from_slice(&map_len.to_le_bytes());
        for (key, value) in &self.map {
            bytes.extend_from_slice(&key.to_le_bytes());
            let value_len = value.len() as u32;
            bytes.extend_from_slice(&value_len.to_le_bytes());
            bytes.extend_from_slice(value.as_bytes());
        }
        bytes
    }

    pub fn deserialize(bytes: &[u8], offset: &mut usize) -> io::Result<Self> {
        // Deserialize the length of the map (stored as a u32)
        let map_len =
            u32::from_le_bytes(bytes[*offset..*offset + 4].try_into().map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "Failed to read map length")
            })?) as usize;
        *offset += 4;

        let mut map = HashMap::with_capacity(map_len);

        // Deserialize each key-value pair in the map
        for _ in 0..map_len {
            let key =
                u32::from_le_bytes(bytes[*offset..*offset + 4].try_into().map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidData, "Failed to read key")
                })?);
            *offset += 4;

            // Read the length of the value string (stored as u32)
            let value_len =
                u32::from_le_bytes(bytes[*offset..*offset + 4].try_into().map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidData, "Failed to read value length")
                })?) as usize;
            *offset += 4;

            // Extract the string value of `value_len` bytes
            let value = String::from_utf8(bytes[*offset..*offset + value_len].to_vec())
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
            *offset += value_len;

            map.insert(key, value);
        }

        Ok(SymbolMap { map })
    }
}

#[cfg(test)]
mod tests {
    use time::macros::datetime;

    use super::*;

    #[test]
    fn test_vendors_into_str() {
        // Test
        let vendor = Vendors::Databento;
        let vendor_str: String = vendor.into();

        // Validate
        assert_eq!("databento", vendor_str);
    }

    #[test]
    fn test_vendors_from_str() {
        // Test
        let vendor_str = "databento";
        let vendor = Vendors::try_from(vendor_str).expect("Error convert to Vendors.");

        // Validate
        assert_eq!(vendor, Vendors::Databento);
    }

    #[test]
    fn test_instrument_first_offsetdatetime() {
        let ticker = "AAPL";
        let name = "Apple Inc.";
        let instrument = Instrument::new(
            None,
            ticker,
            name,
            Vendors::Databento,
            Some("continuous".to_string()),
            Some("GLBX.MDP3".to_string()),
            1730419200000000000,
            1730419200000000000,
            true,
        );
        // Test
        let date = instrument
            .first_available_datetime()
            .expect("Error getting date");

        // Validate
        let expected_date = datetime!(2024-11-01 0:00 UTC); //OffsetDateTime::from("2024-11-01");
        assert_eq!(expected_date, date);
    }

    #[test]
    fn test_instrument_last_offsetdatetime() {
        let ticker = "AAPL";
        let name = "Apple Inc.";
        let instrument = Instrument::new(
            None,
            ticker,
            name,
            Vendors::Databento,
            Some("continuous".to_string()),
            Some("GLBX.MDP3".to_string()),
            1730419200000000000,
            1730419200000000000,
            true,
        );
        // Test
        let date = instrument
            .last_available_datetime()
            .expect("Error getting date");

        // Validate
        let expected_date = datetime!(2024-11-01 0:00 UTC); //OffsetDateTime::from("2024-11-01");
        assert_eq!(expected_date, date);
    }

    #[test]
    fn test_instrument() {
        // Test
        let ticker = "AAPL";
        let name = "Apple Inc.";
        let instrument = Instrument::new(
            None,
            ticker,
            name,
            Vendors::Databento,
            Some("continuous".to_string()),
            Some("GLBX.MDP3".to_string()),
            1,
            1,
            true,
        );

        // Validate
        assert_eq!(instrument.ticker, ticker);
        assert_eq!(instrument.name, name);
        assert_eq!(instrument.instrument_id, None);
    }

    #[test]
    fn test_symbol_map() {
        let appl = "AAPL";
        let tsla = "TSLA";

        // Test
        let mut symbol_map = SymbolMap::new();
        symbol_map.add_instrument(appl, 1);
        symbol_map.add_instrument(tsla, 2);

        // Validate
        let ticker1 = symbol_map.get_instrument_ticker(1).unwrap();
        assert_eq!(&ticker1, appl);

        let ticker2 = symbol_map.get_instrument_ticker(2).unwrap();
        assert_eq!(&ticker2, tsla);
    }
}
