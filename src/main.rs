use clap::{App, Arg};
use env_logger::Builder;
use log::{debug, error, info, warn, LevelFilter};
use std::convert::TryInto;
use std::env;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};

/// Structure used to define information needed to compute checksum on the various LPC processor.
#[derive(Debug)]
struct ProcessorChecksumInfo {
    /// The name of the CPU familly.
    cpu_family: &'static str,
    /// The count of words used for checksum
    words_count: Option<usize>,
    /// The word position of the checksum value.
    resulting_word_position: usize,
}

impl ProcessorChecksumInfo {
    pub fn compute_checksum(&self, firmware_file: &mut File) -> std::io::Result<u32> {
        let mut checksum = 0;
        let mut buffer = Vec::new();
        buffer.resize(self.words_count.unwrap() * std::mem::size_of::<u32>(), 0);

        firmware_file.read_exact(&mut buffer)?;

        let words = buffer
            .chunks(4)
            .map(|value| u32::from_le_bytes(value.try_into().unwrap()))
            .collect::<Vec<u32>>();

        for (i, word) in words.iter().enumerate() {
            if i != self.resulting_word_position {
                checksum += word;
            }
        }

        Ok(0u32.overflowing_sub(checksum).0)
    }
}

static PROCESSOR_CHECKSUM: &[ProcessorChecksumInfo] = &[
    // LPC3 doesn't suppoort checksum validation.
    ProcessorChecksumInfo {
        cpu_family: "LPC3",
        words_count: None,
        resulting_word_position: 0,
    },
    // LPC29 doesn't suppoort checksum validation.
    ProcessorChecksumInfo {
        cpu_family: "LPC29",
        words_count: None,
        resulting_word_position: 0,
    },
    ProcessorChecksumInfo {
        cpu_family: "LPC1",
        words_count: Some(7),
        resulting_word_position: 7,
    },
    ProcessorChecksumInfo {
        cpu_family: "LPC2",
        words_count: Some(8),
        resulting_word_position: 5,
    },
    ProcessorChecksumInfo {
        cpu_family: "LPC4",
        words_count: Some(7),
        resulting_word_position: 7,
    },
    ProcessorChecksumInfo {
        cpu_family: "LPC5",
        words_count: Some(7),
        resulting_word_position: 7,
    },
];

fn get_processor_checksum_info_by_name(cpu_part_number: &str) -> Option<&ProcessorChecksumInfo> {
    for processor in PROCESSOR_CHECKSUM {
        if cpu_part_number.contains(processor.cpu_family) {
            return Some(processor);
        }
    }

    None
}

fn main() -> std::io::Result<()> {
    env::set_var("RUST_LOG", "debug");
    let mut builder = Builder::from_default_env();
    builder.format_timestamp(None);

    let matches = App::new("LPC BootROM checksum calculator")
        .version("1.0")
        .author("Mary")
        .about("Handle LPC BootROM checksum calculation for various LPC processor.")
        .arg(
            Arg::with_name("processor")
                .short("p")
                .long("processor")
                .value_name("PROCESSOR")
                .default_value("LPC1000")
                .help("Define the processor used (e.g. LPC1768, or LPC2103)"),
        )
        .arg(
            Arg::with_name("INPUT")
                .help("Sets the input file to use")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .help("Enable verbose logs"),
        )
        .arg(
            Arg::with_name("display")
                .short("d")
                .long("display")
                .help("Display operation done"),
        )
        .arg(
            Arg::with_name("dry-run")
                .short("n")
                .long("dry-run")
                .help("Do not write the checksum value"),
        )
        .get_matches();

    let verbose = matches.is_present("verbose");
    let display = matches.is_present("display");

    if !display {
        builder.filter(None, LevelFilter::Off);
    }

    if verbose {
        builder.filter(None, LevelFilter::Trace);
    } else if display {
        builder.filter(None, LevelFilter::Info);
    }

    builder.init();

    let processor = matches.value_of("processor").unwrap();
    let input = matches.value_of("INPUT").unwrap();
    let dry_run = matches.is_present("dry-run");

    let mut processor_info_opt = get_processor_checksum_info_by_name(processor);

    if processor_info_opt.is_none() {
        warn!(
            "Unknown processor \"{}\", falling back to LPC1000",
            processor
        );

        processor_info_opt = get_processor_checksum_info_by_name("LPC1000");
    }

    let processor_info = processor_info_opt.unwrap();

    debug!("CPU Familly: {}", processor_info.cpu_family);
    debug!("Firmware file: {}", input);
    debug!("Dry run: {}", dry_run);

    if processor_info.words_count.is_some() {
        let result = OpenOptions::new().read(true).write(true).open(input);
        if let Ok(mut firmware_file) = result {
            let checksum = processor_info.compute_checksum(&mut firmware_file)?;
            info!("Checksum: 0x{:x}", checksum);

            if !dry_run {
                firmware_file.seek(SeekFrom::Start(
                    (processor_info.resulting_word_position * std::mem::size_of::<u32>()) as u64,
                ))?;
                firmware_file.write_all(&checksum.to_le_bytes())?;
            }
        } else {
            error!("Cannot open file {}: {:?}", input, result);
        }
    } else {
        error!("Checksum not supported for {}", processor_info.cpu_family);
    }

    Ok(())
}
