use std::io;
use std::error::Error;
use std::io::prelude::*;
use std::fs::File;

use rustc_serialize::json;
use clap;
use csv;
use itertools::Itertools;
use rust_htslib::bcf;

use libprosic;
use libprosic::model::AlleleFreq;
use libprosic::estimation;
use libprosic::model;

pub fn effective_mutation_rate(matches: &clap::ArgMatches) -> Result<(), Box<Error>> {
    let min_af = value_t!(matches, "min-af", f64).unwrap_or(0.12);
    let max_af = value_t!(matches, "max-af", f64).unwrap_or(0.25);
    let mut reader = csv::Reader::from_reader(io::stdin());
    let freqs = try!(reader.decode().collect::<Result<Vec<f64>, _>>());
    let estimate = estimation::effective_mutation_rate::estimate(freqs.into_iter().filter(|&f| {
        f >= min_af && f <= max_af
    }).map(|f| AlleleFreq(f)));

    // print estimated mutation rate to stdout
    println!("{}", estimate.effective_mutation_rate());

    // if --fit is given, print data visualizing model fit
    if let Some(path) = matches.value_of("fit") {
        let json = json::encode(&estimate).unwrap();
        let mut f = try!(File::create(path));
        try!(f.write_all(json.as_bytes()));
    }
    Ok(())
}


struct DummyEvent {
    pub name: String
}


impl libprosic::Event for DummyEvent {
    fn name(&self) -> &str {
        &self.name
    }
}


pub fn fdr(matches: &clap::ArgMatches) -> Result<(), Box<Error>> {
    let inbcf = matches.value_of("calls").unwrap();
    let outbcf = matches.value_of("output").unwrap_or("-");
    let events = matches.values_of("events").unwrap().map(|e| {
        DummyEvent { name: e.to_owned() }
    }).collect_vec();

    try!(estimation::fdr::annotate(&inbcf, &outbcf, &events));

    Ok(())
}


pub fn fdr_bh(matches: &clap::ArgMatches) -> Result<(), Box<Error>> {
    let call_bcf = matches.value_of("calls").unwrap();
    let null_bcf = matches.value_of("null-calls").unwrap();
    let event = matches.value_of("event").unwrap();
    let vartype = matches.value_of("vartype").unwrap();
    let vartype = match (vartype, value_t!(matches, "min-len", u32).ok(), value_t!(matches, "max-len", u32).ok()) {
        ("SNV", _, _) => model::VariantType::SNV,
        ("INS", Some(min_len), Some(max_len)) => model::VariantType::Insertion(Some(min_len..max_len)),
        ("DEL", Some(min_len), Some(max_len)) => model::VariantType::Deletion(Some(min_len..max_len)),
        ("INS", _, _) => model::VariantType::Insertion(None),
        ("DEL", _, _) => model::VariantType::Deletion(None),
        _ => {
            return Err(Box::new(clap::Error {
                message: "unsupported variant type (supported: SNV, INS, DEL)".to_owned(),
                kind: clap::ErrorKind::InvalidValue,
                info: None
            }));
        }
    };

    let mut call_reader = try!(bcf::Reader::new(&call_bcf));
    let mut null_reader = try!(bcf::Reader::new(&null_bcf));
    let mut writer = io::stdout();
    let event = DummyEvent { name: event.to_owned() };

    try!(estimation::fdr_bh::control_fdr(&mut call_reader, &mut null_reader, &mut writer, &event, &vartype));

    Ok(())
}
