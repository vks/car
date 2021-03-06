

extern crate bzip2;
use self::bzip2::Compression as BzComp;

extern crate flate2;
use self::flate2::Compression as GzComp;

use std::path::Path;
use std::io::{
  self,
  Read,
  Seek,
  SeekFrom
};
use std::fs::OpenOptions;
use std::default::Default;

/*
 * Magic Numbers and Extentions
 *
 * This file attempts to codify and enumerate _all_
 * the various compression algorithms and their signatures
 *
 *  Most of the list is from:https://en.wikipedia.org/wiki/List_of_file_signatures
 *
 * Some are just from reading RFC/Standards:
 * Snappy: https://github.com/google/snappy/blob/master/framing_format.txt
 * XZ: http://tukaani.org/xz/xz-file-format.txt
 * Brotli: https://github.com/madler/brotli/blob/501e6a9d03bcc15f0bc8015f4f36054c30f699ca/br-format-v3.txt
 * Zstd: https://github.com/facebook/zstd
 * Zstd (cont.): https://github.com/facebook/zstd/blob/dev/doc/zstd_compression_format.md
 */


/// Describes the ratio/speed trade off
///
/// More or less these are fixed constants I've eyeballed for
/// the _new_ methods, while for Bzip2/Gzip they're effectively
/// standard values.
///
/// _Any_ input for `Format::Snappy` is ignored as it only has
/// 1 mode of operation. 
///
/// #Brotli
///
///* `Quality::FastLow`: 2
///* `Quality::Default`: 5
///* `Quality::SlowHigh`: 8
///
/// #Zstd
///
///* `Quality::FastLow`: 1
///* `Quality::Default`: 10
///* `Quality::SlowHigh`: 21
///
/// #Lz4
///
///User input is ignored, default value is always used. 
///
/// #Xz
///
///* `Quality::FastLow`: 0
///* `Quality::Default`: 3
///* `Quality::SlowHigh`: 7
#[derive(Copy,Clone,Debug,PartialEq,Eq)]
pub enum Quality {
  Default,
  FastLow,
  SlowHigh
}
impl Quality {
  pub fn into_zstd(self) -> i32 {
    match self {
      Quality::Default => 10,
      Quality::FastLow => 1,
      Quality::SlowHigh => 20
    }
  }
  pub fn into_brotli(self) -> u32 {
    match self {
      Quality::Default => 5,
      Quality::FastLow => 2,
      Quality::SlowHigh => 10,
    }
  }
  pub fn into_xz(self) -> u32 {
    match self {
      Quality::Default => 0,
      Quality::FastLow => 3,
      Quality::SlowHigh => 7,
    }
  }
}
impl Default for Quality {
  
  /// This returns `Quality::Default` which may suprrise you
  fn default() -> Self {
    Quality::Default
  }
}
impl Into<BzComp> for Quality {
  fn into(self) -> BzComp {
    match self {
      Quality::Default => BzComp::Default,
      Quality::FastLow => BzComp::Fastest,
      Quality::SlowHigh => BzComp::Best
    }
  }
}
impl Into<GzComp> for Quality {
  fn into(self) -> GzComp {
    match self {
      Quality::Default => GzComp::Default,
      Quality::FastLow => GzComp::Fast,
      Quality::SlowHigh => GzComp::Best
    }
  }
}


/// Describes the compression algortim we're working with. 
///
/// This can be detected, or set by the user depending on if
/// they're compressing or decompressing.
#[derive(Copy,Clone,Debug,PartialEq,Eq)] 
pub enum Format {
  LZW(Quality),
	LZH(Quality),
	Gzip(Quality),
	Zip7(Quality),
  Bzip2(Quality),
  Xz(Quality),
  Brotli(Quality),
  Lz4(Quality),
  Snappy(Quality),
  Zstd(Quality),
  Tar(Quality),
}
impl Format {

	/// Returns the _suggested_ or _common_ extension for a file format
	/// this isn't a _hard and fast_ rule, some are identical.
	pub fn get_extension(&self) -> &'static str {
		match self {
			&Format::LZW(_) => "z",
			&Format::LZH(_) => "z",
			&Format::Zip7(_) => "7z",
			&Format::Gzip(_) => "gz",
			&Format::Bzip2(_) => "bz2",
			&Format::Xz(_) => "xz",
			&Format::Brotli(_) => "br",
			&Format::Lz4(_) => "lz4",
			&Format::Snappy(_) => "sz",
			&Format::Zstd(_) => "zst",
			&Format::Tar(_) => "tar"
		}
	}

  /// Try to find out the format of a file
  ///
  /// This will attempt to open the file at path and read the first 16bytes
  /// matching that against a known magic number.
  ///
  /// If the file's type is unknown this method will return `Err(InvalidInput)`
  pub fn from_path<P: AsRef<Path>>(p: P) -> io::Result<Format> {
    let mut f = OpenOptions::new().read(true).write(false).create(false).open(p)?;
    let mut v = Vec::with_capacity(16);
    unsafe{ v.set_len(10) };
    f.read_exact(v.as_mut_slice())?;
    let _ = f;
    match what_format(v.as_slice()) {
      Option::Some(f) => Ok(f),
      Option::None => Err(io::Error::new(io::ErrorKind::InvalidInput, "Unsupported file type"))
    }
  }

  /// A slightly more efficient way to find a file's type.
  ///
  /// This method will seek to the start, read 16 bytes, then 
  /// seek back to the start. The goal of this is to avoid multiple open/close cycles
  ///
  /// If the file's type is unknown this method will return `Err(InvalidInput)`
  pub fn from_reader<R:Read+Seek>(r: &mut R) -> io::Result<Format> {
    let _ = r.seek(SeekFrom::Start(0))?;
    let mut v = Vec::with_capacity(16);
    unsafe{ v.set_len(16) };
    r.read_exact(v.as_mut_slice())?;
    let _ = r.seek(SeekFrom::Start(0))?;
    match what_format(v.as_slice()) {
      Option::Some(f) => Ok(f),
      Option::None => Err(io::Error::new(io::ErrorKind::InvalidInput, "Unsupported file type"))
    }
  }
}

/*
 * LOOK I WROTE A NICE PARSER TO DO THIS AND IT BROKE
 * BETWEEN STABLE-MSVC AND STABLE-GNU
 * SO YEAH IT IS RECURSIVE DECENT FUCK OFF
 */
fn what_format(x: &[u8]) -> Option<Format> {
  match &x[0..2] {
    b"\x1F\x9D" => return Some(Format::LZW(Quality::Default)),
    b"\x1F\xA0" => return Some(Format::LZH(Quality::Default)),
    b"\x1F\x8B" => return Some(Format::Gzip(Quality::Default)),
    b"\x30\x30" |
    b"\x20\x00" => return Some(Format::Tar(Quality::Default)),
    _ => { }
  };
  match &x[0..3] {
    b"\x37\x7A\xBC" |
    b"\xAF\x27\x1C" => return Some(Format::Xz(Quality::Default)),
    b"\x42\x5A\x68" => return Some(Format::Bzip2(Quality::Default)),
    b"\x75\x73\x74" |
    b"\x61\x72\x20" |
    b"\x61\x72\x00" => return Some(Format::Tar(Quality::Default)),
    _ => { }
  };
  match &x[0..4] {
    b"\xCE\xB2\xCF\x81" => return Some(Format::Brotli(Quality::Default)),
    b"\x04\x22\x4D\x18" => return Some(Format::Lz4(Quality::Default)),
    b"\xFD\x2F\xB5\x28" => return Some(Format::Zstd(Quality::Default)),
    _ => { }
  };
  match &x[0..6] {
    b"\xFD\x37\x7A\x58\x5A\x00" => return Some(Format::Xz(Quality::Default)),
    _ => { }
  };
  match &x[0..9] {
    b"\xFF\x06\x00\x73\x4E\x61\x50\x70\x59" => return Some(Format::Snappy(Quality::Default)),
    _ => { }
  };
  None
}

			
