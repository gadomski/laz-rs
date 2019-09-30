//! Module with the important struct that people wishing
//! to compress or decompress LAZ data can use
//!
//! It defines the LaszipCompressor & LaszipDecompressor
//! as well as the Laszip VLr data  and how to build it

use std::io::{Read, Seek, SeekFrom, Write};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::compressors::IntegerCompressorBuilder;
use crate::decoders::ArithmeticDecoder;
use crate::decompressors::IntegerDecompressorBuilder;
use crate::encoders::ArithmeticEncoder;
pub use crate::errors::LasZipError;
use crate::las::Point0;
use crate::las::point6::Point6;
use crate::las::rgb::RGB;
use crate::record::{LayeredPointRecordCompressor, LayeredPointRecordDecompressor, RecordCompressor, RecordDecompressor, SequentialPointRecordCompressor, SequentialPointRecordDecompressor};
use crate::las::nir::Nir;

const DEFAULT_CHUNK_SIZE: usize = 50_000;

pub const LASZIP_USER_ID: &'static str = "laszip encoded";
pub const LASZIP_RECORD_ID: u16 = 22204;
pub const LASZIP_DESCRIPTION: &'static str = "http://laszip.org";

#[derive(Debug, Copy, Clone)]
struct Version {
    major: u8,
    minor: u8,
    revision: u16,
}

impl Version {
    fn read_from<R: Read>(src: &mut R) -> std::io::Result<Self> {
        Ok(Self {
            major: src.read_u8()?,
            minor: src.read_u8()?,
            revision: src.read_u16::<LittleEndian>()?,
        })
    }

    fn write_to<W: Write>(&self, dst: &mut W) -> std::io::Result<()> {
        dst.write_u8(self.major)?;
        dst.write_u8(self.minor)?;
        dst.write_u16::<LittleEndian>(self.revision)?;
        Ok(())
    }
}


// The different type of data / fields found in the definition of LAS points
#[derive(Debug, Copy, Clone)]
pub enum LazItemType {
    Byte(u16),
    Point10,
    GpsTime,
    RGB12,
    //WavePacket13,
    Point14,
    RGB14,
    RGBNIR14,
    //WavePacket14,
    Byte14(u16),
}

impl From<LazItemType> for u16 {
    fn from(t: LazItemType) -> Self {
        match t {
            LazItemType::Byte(_) => 0,
            LazItemType::Point10 => 6,
            LazItemType::GpsTime => 7,
            LazItemType::RGB12 => 8,
            //LazItemType::WavePacket13 => 9,
            LazItemType::Point14 => 10,
            LazItemType::RGB14 => 11,
            LazItemType::RGBNIR14 => 12,
            //LazItemType::WavePacket14 => 13,
            LazItemType::Byte14(_) => 14,
        }
    }
}

/// Struct stored as part of the laszip's vlr record_data
///
/// This gives information about the dimension encoded and the version used
/// when encoding the data.
#[derive(Debug, Copy, Clone)]
pub struct LazItem {
    // coded on a u16
    pub(crate) item_type: LazItemType,
    pub(crate) size: u16,
    pub(crate) version: u16,
}

impl LazItem {
    fn read_from<R: Read>(src: &mut R) -> Result<Self, LasZipError> {
        let item_type = src.read_u16::<LittleEndian>()?;
        let size = src.read_u16::<LittleEndian>()?;
        let item_type = match item_type {
            0 => LazItemType::Byte(size),
            6 => LazItemType::Point10,
            7 => LazItemType::GpsTime,
            8 => LazItemType::RGB12,
            //9 => LazItemType::WavePacket13,
            10 => LazItemType::Point14,
            11 => LazItemType::RGB14,
            12 => LazItemType::RGBNIR14,
            //13 => LazItemType::WavePacket14,
            14 => LazItemType::Byte14(size),
            _ => return Err(LasZipError::UnknownLazItem(item_type)),
        };
        Ok(Self {
            item_type,
            size,
            version: src.read_u16::<LittleEndian>()?,
        })
    }

    fn write_to<W: Write>(&self, dst: &mut W) -> std::io::Result<()> {
        dst.write_u16::<LittleEndian>(self.item_type.into())?;
        dst.write_u16::<LittleEndian>(self.size)?;
        dst.write_u16::<LittleEndian>(self.version)?;
        Ok(())
    }
}

pub struct LazItemRecordBuilder {
    items: Vec<LazItemType>,
}

impl LazItemRecordBuilder {
    //TODO What about extrabytes
    pub fn point0() -> Vec<LazItem> {
        vec![
            LazItem {
                item_type: LazItemType::Point10,
                size: Point0::SIZE as u16,
                version: 2,
            }
        ]
    }

    pub fn point1() -> Vec<LazItem> {
        vec![
            LazItem {
                item_type: LazItemType::Point10,
                size: Point0::SIZE as u16,
                version: 2,
            },
            LazItem {
                item_type: LazItemType::GpsTime,
                size: 8,
                version: 2,
            }
        ]
    }

    pub fn point2() -> Vec<LazItem> {
        vec![
            LazItem {
                item_type: LazItemType::Point10,
                size: Point0::SIZE as u16,
                version: 2,
            },
            LazItem {
                item_type: LazItemType::RGB12,
                size: RGB::SIZE as u16,
                version: 2,
            }
        ]
    }

    pub fn point3() -> Vec<LazItem> {
        vec![
            LazItem {
                item_type: LazItemType::Point10,
                size: Point0::SIZE as u16,
                version: 2,
            },
            LazItem {
                item_type: LazItemType::GpsTime,
                size: 8,
                version: 2,
            },
            LazItem {
                item_type: LazItemType::RGB12,
                size: RGB::SIZE as u16,
                version: 2,
            }
        ]
    }

    pub fn point6() -> Vec<LazItem> {
        vec![
            LazItem {
                item_type: LazItemType::Point14,
                size: Point6::SIZE as u16,
                version: 3,
            }
        ]
    }

    pub fn point7() -> Vec<LazItem> {
        vec![
            LazItem {
                item_type: LazItemType::Point14,
                size: Point6::SIZE as u16,
                version: 3,
            },
            LazItem {
                item_type: LazItemType::RGB14,
                size: RGB::SIZE as u16,
                version: 3,
            }
        ]
    }

    pub fn point8() -> Vec<LazItem> {
        vec![
            LazItem {
                item_type: LazItemType::Point14,
                size: Point6::SIZE as u16,
                version: 3,
            },
            LazItem {
                item_type: LazItemType::RGBNIR14,
                size: (RGB::SIZE + Nir::SIZE ) as u16 ,
                version: 3,
            }
        ]
    }

    pub fn new() -> Self {
        Self { items: vec![] }
    }

    pub fn add_item(&mut self, item_type: LazItemType) -> &mut Self {
        self.items.push(item_type);
        self
    }

    pub fn build(&self) -> Vec<LazItem> {
        self.items
            .iter()
            .map(|item_type| {
                let size = match *item_type {
                    LazItemType::Byte(n) => n,
                    LazItemType::Point10 => 20,
                    LazItemType::GpsTime => 8,
                    LazItemType::RGB12 => 6,
                    LazItemType::Point14 => Point6::SIZE as u16,
                    LazItemType::RGB14 => 6,
                    LazItemType::RGBNIR14 => (RGB::SIZE + Nir::SIZE) as u16,
                    LazItemType::Byte14(n) => n,
                };
                let version = match item_type {
                    LazItemType::Byte(_) => 2,
                    LazItemType::Point10 => 2,
                    LazItemType::GpsTime => 2,
                    LazItemType::RGB12 => 2,
                    LazItemType::Point14 => 3,
                    LazItemType::RGB14 => 3,
                    LazItemType::RGBNIR14 => 3,
                    LazItemType::Byte14(_) => 3,
                };
                LazItem {
                    item_type: *item_type,
                    size,
                    version,
                }
            })
            .collect()
    }
}

fn read_laz_items_from<R: Read>(mut src: &mut R) -> Result<Vec<LazItem>, LasZipError> {
    let num_items = src.read_u16::<LittleEndian>()?;
    let mut items = Vec::<LazItem>::with_capacity(num_items as usize);
    for _ in 0..num_items {
        items.push(LazItem::read_from(&mut src)?)
    }
    Ok(items)
}

fn write_laz_items_to<W: Write>(laz_items: &Vec<LazItem>, mut dst: &mut W) -> std::io::Result<()> {
    dst.write_u16::<LittleEndian>(laz_items.len() as u16)?;
    for item in laz_items {
        item.write_to(&mut dst)?;
    }
    Ok(())
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum CompressorType {
    // TODO might need a better name
    None = 0,
    /// No chunks, or rather only 1 chunk with all the points
    PointWise = 1,
    /// Compress points into chunks with chunk_size points in each chunks
    PointWiseChunked = 2,
    /// Compress points into chunk, but also separate the different point dimension / fields
    /// into layers. This CompressorType is only use for point 6,7,8,9,10
    LayeredChunked = 3,
}

impl CompressorType {
    fn from_u16(t: u16) -> Option<Self> {
        match t {
            0 => Some(CompressorType::None),
            1 => Some(CompressorType::PointWise),
            2 => Some(CompressorType::PointWiseChunked),
            3 => Some(CompressorType::LayeredChunked),
            _ => None,
        }
    }
}

impl Default for CompressorType {
    fn default() -> Self {
        CompressorType::PointWiseChunked
    }
}


/// The data stored in the record_data of the Laszip Vlr
#[derive(Debug, Clone)]
pub struct LazVlr {
    // coded on u16
    compressor: CompressorType,
    // 0 means ArithmeticCoder, its the only choice
    coder: u16,

    version: Version,
    options: u32,
    /// Number of points per chunk
    chunk_size: u32,

    // -1 if unused
    number_of_special_evlrs: i64,
    // -1 if unused
    offset_to_special_evlrs: i64,

    items: Vec<LazItem>,
}

impl LazVlr {
    // TODO should impl default instead
    pub fn new() -> Self {
        Self {
            compressor: CompressorType::default(),
            coder: 0,
            version: Version {
                major: 2,
                minor: 2,
                revision: 0,
            },
            options: 0,
            chunk_size: DEFAULT_CHUNK_SIZE as u32,
            number_of_special_evlrs: -1,
            offset_to_special_evlrs: -1,
            items: vec![],
        }
    }

    pub fn from_laz_items(items: Vec<LazItem>) -> Self {
        let mut me = Self::new();
        me.items = items;
        me
    }

    /// Tries to read the Vlr information from the record_data buffer
    pub fn from_buffer(record_data: &[u8]) -> Result<Self, LasZipError> {
        let mut cursor = std::io::Cursor::new(record_data);
        Self::read_from(&mut cursor)
    }

    /// Tries to read the Vlr information from the record_data source
    pub fn read_from<R: Read>(mut src: &mut R) -> Result<Self, LasZipError> {
        let compressor_type = src.read_u16::<LittleEndian>()?;
        let compressor = match CompressorType::from_u16(compressor_type) {
            Some(c) => c,
            None => return Err(LasZipError::UnknownCompressorType(compressor_type)),
        };

        Ok(Self {
            compressor,
            coder: src.read_u16::<LittleEndian>()?,
            version: Version::read_from(&mut src)?,
            options: src.read_u32::<LittleEndian>()?,
            chunk_size: src.read_u32::<LittleEndian>()?,
            number_of_special_evlrs: src.read_i64::<LittleEndian>()?,
            offset_to_special_evlrs: src.read_i64::<LittleEndian>()?,
            items: read_laz_items_from(&mut src)?,
        })
    }

    /// Writes the Vlr to the source, this only write the 'record_data' the
    /// header should be written before-hand
    pub fn write_to<W: Write>(&self, mut dst: &mut W) -> std::io::Result<()> {
        dst.write_u16::<LittleEndian>(self.compressor as u16)?;
        dst.write_u16::<LittleEndian>(self.coder)?;
        self.version.write_to(&mut dst)?;
        dst.write_u32::<LittleEndian>(self.options)?;
        dst.write_u32::<LittleEndian>(self.chunk_size)?;
        dst.write_i64::<LittleEndian>(self.number_of_special_evlrs)?;
        dst.write_i64::<LittleEndian>(self.offset_to_special_evlrs)?;
        write_laz_items_to(&self.items, &mut dst)?;
        Ok(())
    }

    pub fn chunk_size(&self) -> u32 {
        self.chunk_size
    }

    pub fn items(&self) -> &Vec<LazItem> {
        &self.items
    }

    /// Returns the sum of the size of the laz_items, which should correspond to the
    /// expected size of points (uncompressed).
    pub fn items_size(&self) -> u64 {
        u64::from(self.items.iter().map(|item| item.size).sum::<u16>())
    }
}

impl Default for LazVlr {
    fn default() -> Self {
        Self {
            compressor: Default::default(),
            coder: 0,
            version: Version {
                major: 2,
                minor: 2,
                revision: 0,
            },
            options: 0,
            chunk_size: DEFAULT_CHUNK_SIZE as u32,
            number_of_special_evlrs: -1,
            offset_to_special_evlrs: -1,
            items: vec![],
        }
    }
}

pub struct LazVlrBuilder {
    laz_vlr: LazVlr,
}

impl LazVlrBuilder {
    pub fn new() -> Self {
        Self {
            laz_vlr: Default::default(),
        }
    }

    pub fn with_chunk_size(mut self, chunk_size: u32) -> Self {
        self.laz_vlr.chunk_size = chunk_size;
        self
    }

    pub fn with_laz_items(mut self, laz_items: Vec<LazItem>) -> Self {
        self.laz_vlr.items = laz_items;
        self
    }

    pub fn build(self) -> LazVlr {
        self.laz_vlr
    }
}

//TODO fix unwraps
pub fn record_decompressor_from_laz_items<R: Read + Seek + 'static>(
    items: &Vec<LazItem>,
    input: R,
) -> Box<dyn RecordDecompressor<R>> {
    let first_version = items[0].version;
    if !items.iter().all(|item| item.version == first_version) {
        // Technically we could mix version 1&2 and 3&4
        // we just cannot mix non-layered decompressor and layered-decompressor
        panic!("All laz items must have save version");
    }

    match first_version {
        1 | 2 => {
            let mut decompressor = SequentialPointRecordDecompressor::new(input);
            decompressor.set_fields_from(items).unwrap();
            Box::new(decompressor)
        }
        3 | 4 => {
            let mut decompressor = LayeredPointRecordDecompressor::new(input);
            decompressor.set_fields_from(items).unwrap();
            Box::new(decompressor)
        }
        _ => panic!("Unknown laz item version {}", first_version),
    }
}


pub fn record_compressor_from_laz_items<W: Write + 'static>(items: &Vec<LazItem>, output: W) -> Box<dyn RecordCompressor<W>> {
    let first_version = items[0].version;
    if !items.iter().all(|item| item.version == first_version) {
        // Technically we could mix version 1&2 and 3&4
        // we just cannot mix non-layered decompressor and layered-decompressor
        panic!("All laz items must have save version");
    }

    match first_version {
        1 | 2 => {
            let mut compressor = SequentialPointRecordCompressor::new(output);
            compressor.set_fields_from(items).unwrap();
            Box::new(compressor)
        }
        3 | 4 => {
            let mut compressor = LayeredPointRecordCompressor::new(output);
            compressor.set_fields_from(items).unwrap();
            Box::new(compressor)
        }
        _ => panic!("Unknown laz item version {}", first_version),
    }
}

//TODO possible to make the Seek trait optional ?
/// Struct that handles the decompression of the points inside the source
pub struct LasZipDecompressor<R: Read + Seek + Sized + 'static> {
    vlr: LazVlr,
    record_decompressor: Box<dyn RecordDecompressor<R>>,
    chunk_points_read: u32,
    offset_to_chunk_table: i64,
    data_start: u64,
    chunk_table: Option<Vec<u64>>,
}

impl<R: Read + Seek + Sized + 'static> LasZipDecompressor<R> {
    pub fn new_with_record_data(
        source: R,
        laszip_vlr_record_data: &[u8],
    ) -> Result<Self, LasZipError> {
        let vlr = LazVlr::from_buffer(laszip_vlr_record_data)?;
        Self::new(source, vlr)
    }

    pub fn new(mut source: R, vlr: LazVlr) -> Result<Self, LasZipError> {
        if vlr.compressor != CompressorType::PointWiseChunked
            && vlr.compressor != CompressorType::LayeredChunked
        {
            return Err(LasZipError::UnsupportedCompressorType(vlr.compressor));
        }

        let offset_to_chunk_table = source.read_i64::<LittleEndian>()?;
        let data_start = source.seek(SeekFrom::Current(0))?;
        let record_decompressor = record_decompressor_from_laz_items(&vlr.items, source);

        Ok(Self {
            vlr,
            record_decompressor,
            chunk_points_read: 0,
            offset_to_chunk_table,
            data_start,
            chunk_table: None,
        })
    }

    /// Decompress the next point and write the uncompressed data to the out buffer.
    ///
    /// - The buffer should have at least enough byte to store the decompressed data
    /// - The data is written in the buffer exactly as it would have been in a LAS File
    ///     in Little Endian order,
    pub fn decompress_one(&mut self, mut out: &mut [u8]) -> std::io::Result<()> {
        if self.chunk_points_read == self.vlr.chunk_size {
            self.reset_for_new_chunk();
        }
        self.record_decompressor.decompress_next(&mut out)?;
        self.chunk_points_read += 1;
        Ok(())
    }

    pub fn into_stream(self) -> R {
        self.record_decompressor.box_into_stream()
    }

    // FIXME Seeking in Layered Compressed data is untested, make sure it works
    pub fn seek(&mut self, point_idx: u64) -> std::io::Result<()> {
        if let Some(chunk_table) = &self.chunk_table {
            let chunk_of_point = point_idx / self.vlr.chunk_size as u64;
            let delta = point_idx % self.vlr.chunk_size as u64;

            if chunk_of_point == (chunk_table.len() - 1) as u64 {
                // the requested point fall into the last chunk,
                // but that does not mean that the point exists
                // so we have to be careful, we will do as we would normally,
                // but if we reach the chunk_table_offset that means the requested
                // point is out ouf bounds so will just seek to the end cf(the else in the if let below)
                // we do this to avoid decompressing data (ie the chunk table) thinking its a record

                if self.offset_to_chunk_table == -1 {
                    // If the offset is still -1 it means the chunk table could not
                    // be read :thinking:
                    unreachable!("unexpected offset to chunk table");
                }
                let mut tmp_out = vec![0u8; self.record_decompressor.record_size()];
                self.record_decompressor
                    .borrow_stream_mut()
                    .seek(SeekFrom::Start(chunk_table[chunk_of_point as usize] as u64))?;

                self.reset_for_new_chunk();

                for _i in 0..delta {
                    self.decompress_one(&mut tmp_out)?;
                    let current_pos = self
                        .record_decompressor
                        .borrow_stream_mut()
                        .seek(SeekFrom::Current(0))?;

                    if current_pos >= self.offset_to_chunk_table as u64 {
                        self.record_decompressor
                            .borrow_stream_mut()
                            .seek(SeekFrom::End(0))?;
                        return Ok(());
                    }
                }
            } else if let Some(start_of_chunk) = chunk_table.get(chunk_of_point as usize) {
                self.record_decompressor
                    .borrow_stream_mut()
                    .seek(SeekFrom::Start(*start_of_chunk as u64))?;
                self.reset_for_new_chunk();
                let mut tmp_out = vec![0u8; self.record_decompressor.record_size()];

                for _i in 0..delta {
                    self.decompress_one(&mut tmp_out)?;
                }
            } else {
                // the requested point it out of bounds (ie higher than the number of
                // points compressed)

                // Seek to the end so that the next call to decompress causes en error
                // like "Failed to fill whole buffer (seeking past end is allowed by the Seek Trait)
                self.record_decompressor
                    .borrow_stream_mut()
                    .seek(SeekFrom::End(0))?;
            }
            Ok(())
        } else {
            self.read_chunk_table()?;
            self.seek(point_idx)
        }
    }

    fn reset_for_new_chunk(&mut self) {
        self.chunk_points_read = 0;
        self.record_decompressor.reset();
        //we can safely unwrap here, as set_field would have failed in the ::new()
        self.record_decompressor
            .set_fields_from(&self.vlr.items)
            .unwrap();
    }

    fn read_chunk_table(&mut self) -> std::io::Result<()> {
        let mut stream = self.record_decompressor.borrow_stream_mut();
        let current_pos = stream.seek(SeekFrom::Current(0))?;
        if self.offset_to_chunk_table == -1 {
            // Compressor was writing to non seekable stream
            stream.seek(SeekFrom::End(-8))?;
            self.offset_to_chunk_table = stream.read_i64::<LittleEndian>()?;
        }
        stream.seek(SeekFrom::Start(self.offset_to_chunk_table as u64))?;

        let _version = stream.read_u32::<LittleEndian>()?;
        let number_of_chunks = stream.read_u32::<LittleEndian>()?;
        let mut chunk_sizes = vec![0u64; number_of_chunks as usize];

        let mut decompressor = IntegerDecompressorBuilder::new()
            .bits(32)
            .contexts(2)
            .build_initialized();
        let mut decoder = ArithmeticDecoder::new(&mut stream);
        decoder.read_init_bytes()?;
        for i in 1..=number_of_chunks {
            chunk_sizes[(i - 1) as usize] = decompressor.decompress(
                &mut decoder,
                if i > 1 {
                    chunk_sizes[(i - 2) as usize]
                } else {
                    0
                } as i32,
                1,
            )? as u64;
        }
        let mut chunk_starts = vec![0u64; number_of_chunks as usize];
        chunk_starts[0] = self.data_start;
        for i in 1..number_of_chunks {
            chunk_starts[i as usize] =
                chunk_sizes[(i - 1) as usize] + chunk_starts[(i - 1) as usize];
            /*if (chunk_starts[i] <= chunk_starts[i-1]) {
                //err
            }*/
        }
        self.chunk_table = Some(chunk_starts);
        stream.seek(SeekFrom::Start(current_pos))?;
        Ok(())
    }
}
/// Struct that handles the compression of the points into the given destination
pub struct LasZipCompressor<W: Write> {
    vlr: LazVlr,
    record_compressor: Box<dyn RecordCompressor<W>>,
    first_point: bool,
    chunk_point_written: u32,
    chunk_sizes: Vec<usize>,
    last_chunk_pos: u64,
    start_pos: u64,
}

// FIXME What laszip does for the chunk table is: if stream is not seekable: chunk table offset is -1
//  write the chunk table  as usual then after (so at the end of the stream write the chunk table
//  that means also support non seekable stream this is waht we have to do

impl<W: Write + Seek + 'static> LasZipCompressor<W> {
    pub fn from_laz_items(output: W, items: Vec<LazItem>) -> Result<Self, LasZipError> {
        let vlr = LazVlr::from_laz_items(items);
        Self::from_laz_vlr(output, vlr)
    }

    pub fn from_laz_vlr(output: W, vlr: LazVlr) -> Result<Self, LasZipError> {
        let record_compressor = record_compressor_from_laz_items(&vlr.items, output);
        Ok(Self {
            vlr,
            record_compressor,
            first_point: true,
            chunk_point_written: 0,
            chunk_sizes: vec![],
            last_chunk_pos: 0,
            start_pos: 0,
        })
    }

    /// Compress the point and write the compressed data to the destination given when
    /// the compressor was constructed
    ///
    /// The data is written in the buffer is expected to be exactly
    /// as it would have been in a LAS File, that is:
    ///
    /// - The fields/dimensions are in the same order than the LAS spec says
    /// - The data in the buffer is in Little Endian order
    pub fn compress_one(&mut self, input: &[u8]) -> std::io::Result<()> {
        if self.first_point {
            let stream = self.record_compressor.borrow_stream_mut();
            self.start_pos = stream.seek(SeekFrom::Current(0))?;
            stream.write_i64::<LittleEndian>(-1)?;
            self.last_chunk_pos = self.start_pos + std::mem::size_of::<i64>() as u64;
            self.first_point = false;
        }

        if self.chunk_point_written == self.vlr.chunk_size {
            self.record_compressor.done()?;
            self.record_compressor.reset();
            self.record_compressor
                .set_fields_from(&self.vlr.items)
                .unwrap();
            self.update_chunk_table()?;
            self.chunk_point_written = 0;
        }

        self.record_compressor.compress_next(&input)?;
        self.chunk_point_written += 1;
        Ok(())
    }

    pub fn done(&mut self) -> std::io::Result<()> {
        self.record_compressor.done()?;
        self.update_chunk_table()?;
        self.update_chunk_table_offset()?;
        self.write_chunk_table()?;
        Ok(())
    }

    pub fn vlr(&self) -> &LazVlr {
        &self.vlr
    }

    pub fn into_stream(self) -> W {
        self.record_compressor.box_into_stream()
    }

    pub fn get_mut(&mut self) -> &mut W {
        self.record_compressor.borrow_stream_mut()
    }

    fn write_chunk_table(&mut self) -> std::io::Result<()> {
        // Write header
        let mut stream = self.record_compressor.borrow_stream_mut();
        stream.write_u32::<LittleEndian>(0)?;
        stream.write_u32::<LittleEndian>(self.chunk_sizes.len() as u32)?;

        let mut encoder = ArithmeticEncoder::new(&mut stream);
        let mut compressor = IntegerCompressorBuilder::new()
            .bits(32)
            .contexts(2)
            .build_initialized();

        let mut predictor = 0;
        for chunk_size in &self.chunk_sizes {
            compressor.compress(&mut encoder, predictor, (*chunk_size) as i32, 1)?;
            predictor = (*chunk_size) as i32;
        }
        encoder.done()?;
        Ok(())
    }

    fn update_chunk_table(&mut self) -> std::io::Result<()> {
        let current_pos = self
            .record_compressor
            .borrow_stream_mut()
            .seek(SeekFrom::Current(0))?;
        self.chunk_sizes
            .push((current_pos - self.last_chunk_pos) as usize);
        self.last_chunk_pos = current_pos;
        Ok(())
    }

    fn update_chunk_table_offset(&mut self) -> std::io::Result<()> {
        let stream = self.record_compressor.borrow_stream_mut();
        let start_of_chunk_table_pos = stream.seek(SeekFrom::Current(0))?;
        stream.seek(SeekFrom::Start(self.start_pos))?;
        stream.write_i64::<LittleEndian>(start_of_chunk_table_pos as i64)?;
        stream.seek(SeekFrom::Start(start_of_chunk_table_pos))?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn test_can_write_laz_vlr() {
        let vlr = LazVlr::new();
        let mut out = Cursor::new(Vec::<u8>::new());
        vlr.write_to(&mut out).unwrap();
    }

    #[test]
    fn test_create_laz_items() {
        assert_eq!(
            LazItemRecordBuilder::new()
                .add_item(LazItemType::Point10)
                .build()
                .len(),
            1
        );
    }
}
