use mbn::enums::RType;
use mbn::record_enum::RecordEnum;
use mbn::record_ref::RecordRef;
use mbn::records::{
    BboMsg, BidAskPair, Mbp1Msg, OhlcvMsg, Record, RecordHeader, TbboMsg, TradeMsg,
};

use std::os::raw::c_char;

use std::mem::ManuallyDrop;

pub trait ToRecordRef<'a> {
    fn to_record_ref(&'a self) -> RecordRef<'a>;
}

#[repr(C)]
pub union RecordData {
    mbp1: ManuallyDrop<Mbp1Msg>,
    ohlcv: ManuallyDrop<OhlcvMsg>,
    trade: ManuallyDrop<TradeMsg>,
    tbbo: ManuallyDrop<Mbp1Msg>,
    bbo: ManuallyDrop<BboMsg>,
}

// Implement From<RecordEnum> for RecordData
impl From<RecordEnum> for RecordData {
    fn from(value: RecordEnum) -> Self {
        match value {
            RecordEnum::Mbp1(msg) => RecordData {
                mbp1: ManuallyDrop::new(msg),
            },
            RecordEnum::Ohlcv(msg) => RecordData {
                ohlcv: ManuallyDrop::new(msg),
            },
            RecordEnum::Trade(msg) => RecordData {
                trade: ManuallyDrop::new(msg),
            },
            RecordEnum::Bbo(msg) => RecordData {
                bbo: ManuallyDrop::new(msg),
            },
            RecordEnum::Tbbo(msg) => RecordData {
                tbbo: ManuallyDrop::new(msg),
            },
        }
    }
}

impl<'a> ToRecordRef<'a> for RecordData {
    fn to_record_ref(&'a self) -> RecordRef<'a> {
        unsafe {
            let record_type = self.mbp1.header().rtype();

            match record_type {
                RType::Mbp1 => RecordRef::from(&*self.mbp1),
                RType::Ohlcv => RecordRef::from(&*self.ohlcv),
                RType::Bbo => RecordRef::from(&*self.bbo),
                RType::Tbbo => RecordRef::from(&*self.tbbo),
                RType::Trade => RecordRef::from(&*self.trade),
            }
        }
    }
}

// Get the type (safe across all variants)
#[no_mangle]
pub extern "C" fn get_rtype(record: *const RecordData) -> RType {
    unsafe { (*record).mbp1.header().rtype() }
}

#[no_mangle]
pub extern "C" fn get_mbp1(record: *const RecordData) -> *const Mbp1Msg {
    if record.is_null() {
        return std::ptr::null(); // Return null if input pointer is null
    }

    let rtype = get_rtype(record);
    println!("{}", rtype);

    if rtype == RType::Mbp1 {
        return unsafe { &(*record).mbp1 as *const ManuallyDrop<Mbp1Msg> as *const Mbp1Msg };
    } else {
        return std::ptr::null(); // Return null if input pointer is null
    }
}
#[no_mangle]
pub extern "C" fn get_ohlcv(record: *const RecordData) -> *const OhlcvMsg {
    if record.is_null() {
        return std::ptr::null(); // Return null if input pointer is null
    }

    let rtype = get_rtype(record);

    if rtype == RType::Ohlcv {
        return unsafe { &(*record).ohlcv as *const ManuallyDrop<OhlcvMsg> as *const OhlcvMsg };
    } else {
        return std::ptr::null(); // Return null if input pointer is null
    }
}
#[no_mangle]
pub extern "C" fn get_trade(record: *const RecordData) -> *const TradeMsg {
    if record.is_null() {
        return std::ptr::null(); // Return null if input pointer is null
    }

    let rtype = get_rtype(record);

    if rtype == RType::Trade {
        return unsafe { &(*record).trade as *const ManuallyDrop<TradeMsg> as *const TradeMsg };
    } else {
        return std::ptr::null(); // Return null if input pointer is null
    }
}
#[no_mangle]
pub extern "C" fn get_tbbo(record: *const RecordData) -> *const TbboMsg {
    if record.is_null() {
        return std::ptr::null(); // Return null if input pointer is null
    }

    let rtype = get_rtype(record);

    if rtype == RType::Tbbo {
        return unsafe { &(*record).tbbo as *const ManuallyDrop<TbboMsg> as *const TbboMsg };
    } else {
        return std::ptr::null(); // Return null if input pointer is null
    }
}

#[no_mangle]
pub extern "C" fn get_bbo(record: *const RecordData) -> *const BboMsg {
    if record.is_null() {
        return std::ptr::null(); // Return null if input pointer is null
    }

    let rtype = get_rtype(record);

    if rtype == RType::Bbo {
        return unsafe { &(*record).bbo as *const ManuallyDrop<BboMsg> as *const BboMsg };
    } else {
        return std::ptr::null(); // Return null if input pointer is null
    }
}

#[no_mangle]
pub extern "C" fn create_mbp1(
    instrument_id: u32,
    ts_event: u64,
    price: i64,
    size: u32,
    action: c_char,
    side: c_char,
    depth: u8,
    flags: u8,
    ts_recv: u64,
    ts_in_delta: i32,
    sequence: u32,
    discriminator: u32,
    bid_px: i64,
    ask_px: i64,
    bid_sz: u32,
    ask_sz: u32,
    bid_ct: u32,
    ask_ct: u32,
) -> Mbp1Msg {
    // assert!(!levels.is_null(), "Levels pointer must not be null");

    // SAFELY consume the C pointer
    let bidask: BidAskPair = BidAskPair {
        bid_px,
        ask_px,
        bid_sz,
        ask_sz,
        bid_ct,
        ask_ct,
    }; //unsafe { [*Box::from_raw(levels); 1] };

    Mbp1Msg {
        hd: RecordHeader::new::<Mbp1Msg>(instrument_id, ts_event),
        price,
        size,
        action,
        side,
        depth,
        flags,
        ts_recv,
        ts_in_delta,
        sequence,
        discriminator,
        levels: [bidask; 1],
    }
}

#[no_mangle]
pub extern "C" fn create_ohlcv(
    instrument_id: u32,
    ts_event: u64,
    open: i64,
    high: i64,
    low: i64,
    close: i64,
    volume: u64,
) -> OhlcvMsg {
    OhlcvMsg {
        hd: RecordHeader::new::<OhlcvMsg>(instrument_id, ts_event),
        open,
        high,
        low,
        close,
        volume,
    }
}

#[no_mangle]
pub extern "C" fn create_trade(
    instrument_id: u32,
    ts_event: u64,
    price: i64,
    size: u32,
    action: c_char,
    side: c_char,
    depth: u8,
    flags: u8,
    ts_recv: u64,
    ts_in_delta: i32,
    sequence: u32,
) -> TradeMsg {
    TradeMsg {
        hd: RecordHeader::new::<TradeMsg>(instrument_id, ts_event),
        price,
        size,
        action,
        side,
        depth,
        flags,
        ts_recv,
        ts_in_delta,
        sequence,
    }
}

#[no_mangle]
pub extern "C" fn create_bbo(
    instrument_id: u32,
    ts_event: u64,
    price: i64,
    size: u32,
    side: c_char,
    flags: u8,
    ts_recv: u64,
    sequence: u32,
    bid_px: i64,
    ask_px: i64,
    bid_sz: u32,
    ask_sz: u32,
    bid_ct: u32,
    ask_ct: u32,
) -> BboMsg {
    // SAFELY consume the C pointer
    let bidask: BidAskPair = BidAskPair {
        bid_px,
        ask_px,
        bid_sz,
        ask_sz,
        bid_ct,
        ask_ct,
    };
    BboMsg {
        hd: RecordHeader::new::<BboMsg>(instrument_id, ts_event),
        price,
        size,
        side,
        flags,
        ts_recv,
        sequence,
        levels: [bidask; 1],
    }
}
