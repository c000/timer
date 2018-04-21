extern crate chrono;
extern crate term;
extern crate ctrlc;

use chrono::Duration;
use chrono::offset::Utc;
use std::env::args;
use std::io;
use std::io::Write;
use std::thread;

fn parse_hms<T>(args: &mut T)
  -> Result<Duration, std::num::ParseIntError>
  where T: Iterator<Item=String> {
    let fs = [Duration::hours,
              Duration::minutes,
              Duration::seconds];
    let ds = fs.into_iter()
        .map(|f| args.next().unwrap_or(String::from("0")).parse().map(|x| f(x)));

    let mut acc = Duration::zero();
    for d in ds {
        acc = acc + try!(d)
    }
    Ok(acc)
}

fn write_passed_time<T>(buf: &mut T, passed: Duration, entire: Duration)
  -> Result<(), std::io::Error>
  where T: Write {
    let rest = entire - passed;
    let milli = rest.num_milliseconds() as f64;
    let secs  = milli.abs() / 1000. % 60.;
    let mins  = (milli.abs() / 60_000.).floor() % 60.;
    let hours = (milli.abs() / 3600_000.).floor();
    let rational = passed.num_milliseconds() as f64 / entire.num_milliseconds() as f64;

    write!(buf,
        "\r{:.0}% {:.4} | {}{:02.0}:{:02.0}:{:04.1} ",
        (rational*100.).min(100.).floor(),
        rational,
        if milli < 0. {"-"} else {" "},
        hours,
        mins,
        secs)
}

fn print_usage_and_exit<S, T>(program: S, e: T) -> !
  where S: std::borrow::Borrow<str>,
        T: std::fmt::Display {
    write!(io::stderr(),
        "Usage:\n\t{} [HOUR [MINUTE [SECOND]]]\n{}",
        program.borrow(),
        e)
        .unwrap_or(());
    std::process::exit(1);
}

fn main() {
    let mut a = args();
    let program_name = a.next().unwrap();
    let dur = parse_hms(&mut a)
        .unwrap_or_else(|e|
            print_usage_and_exit(program_name.as_ref(), e)
        );
    // Check the rest arguments
    if let Some(_) = a.next() {
        print_usage_and_exit(program_name.as_ref(), "");
    }

    let (tx, rx) = std::sync::mpsc::channel();

    ctrlc::set_handler(move || {
        tx.send(()).unwrap();
    }).unwrap();

    let handle = thread::spawn(move || {
        let start = Utc::now();
        let mut t = term::stdout().unwrap();

        loop {
            let passed = Utc::now() - start;
            let milli = (dur - passed).num_milliseconds() as f64;
            if milli < 0. {
                t.bg(term::color::RED).unwrap();
            }
            write_passed_time(&mut t, passed, dur).unwrap();
            io::stdout().flush().unwrap();
            if rx.try_recv().is_ok() {
                break;
            }
            thread::sleep(std::time::Duration::from_millis(100));
        }
        t.reset().unwrap();
    });

    handle.join().unwrap();
    println!("");
}
