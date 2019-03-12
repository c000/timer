extern crate chrono;
extern crate console;
extern crate ctrlc;

use chrono::offset::Utc;
use chrono::Duration;
use std::borrow::Cow;
use std::env::args;
use std::io;
use std::io::Write;
use std::thread;

use console::Term;

fn parse_hms<T>(args: &mut T) -> Result<Duration, std::num::ParseIntError>
where
    T: Iterator<Item = String>,
{
    let fs = [Duration::hours, Duration::minutes, Duration::seconds];
    let ds = fs.into_iter().map(|f| {
        args.next()
            .unwrap_or(String::from("0"))
            .parse()
            .map(|x| f(x))
    });

    let mut acc = Duration::zero();
    for d in ds {
        acc = acc + try!(d)
    }
    Ok(acc)
}

struct PassedTime {
    milli: f64,
    secs: f64,
    mins: f64,
    hours: f64,
    rational: f64,
}

impl PassedTime {
    pub fn new(passed: Duration, entire: Duration) -> PassedTime {
        let rest = entire - passed;
        let milli = rest.num_milliseconds() as f64;
        PassedTime {
            milli: milli,
            secs: milli.abs() / 1000. % 60.,
            mins: (milli.abs() / 60_000.).floor() % 60.,
            hours: (milli.abs() / 3600_000.).floor(),
            rational: passed.num_milliseconds() as f64 / entire.num_milliseconds() as f64,
        }
    }

    pub fn format_passed(&self) -> String {
        format!(
            "{:.0}% {:.4} | {}{:02.0}:{:02.0}:{:04.1}",
            (self.rational * 100.).min(100.).floor(),
            self.rational,
            if self.milli < 0. { "-" } else { " " },
            self.hours,
            self.mins,
            self.secs
        )
    }

    pub fn format_progress(&self, length: usize) -> Cow<'static, str> {
        if length < 2 {
            return "".into();
        }
        let inner_size = length - 2;
        let progress = (self.rational * inner_size as f64) as usize;
        let progress_str = std::iter::repeat('=')
            .take(progress)
            .chain(std::iter::once('>'))
            .chain(std::iter::repeat(' '));
        format!("[{}]", progress_str.take(inner_size).collect::<String>()).into()
    }

    pub fn format_term(&self, t: &Term) -> std::result::Result<Vec<u8>, Box<std::error::Error>> {
        let mut buf = std::vec::Vec::new();
        let (_, width) = t.size();
        let mut style = console::Style::new();
        if 1. <= self.rational {
            style = style.on_red();
        }
        let passed_str = self.format_passed();
        let progress_str = self.format_progress(width as usize - passed_str.len() - 1);
        write!(buf, "\r{} {}", passed_str, style.apply_to(progress_str))?;
        Ok(buf)
    }
}

fn print_usage_and_exit<S, T>(program: S, e: T) -> !
where
    S: std::borrow::Borrow<str>,
    T: std::fmt::Display,
{
    write!(
        io::stderr(),
        "Usage:\n\t{} [HOUR [MINUTE [SECOND]]]\n{}",
        program.borrow(),
        e
    )
    .unwrap_or(());
    std::process::exit(1);
}

fn main() {
    let mut a = args();
    let program_name = a.next().unwrap();
    let dur = parse_hms(&mut a).unwrap_or_else(|e| print_usage_and_exit(program_name.as_ref(), e));
    // Check the rest arguments
    if let Some(_) = a.next() {
        print_usage_and_exit(program_name.as_ref(), "");
    }

    let (tx, rx) = std::sync::mpsc::channel();

    ctrlc::set_handler(move || {
        tx.send(()).unwrap();
    })
    .unwrap();

    let handle = thread::spawn(move || {
        let start = Utc::now();
        let mut t = Term::stdout();

        loop {
            let passed = Utc::now() - start;
            let passed = PassedTime::new(passed, dur);
            let buf = passed.format_term(&t).unwrap();
            t.write(&buf).unwrap();
            t.flush().unwrap();
            if rx.try_recv().is_ok() {
                break;
            }
            thread::sleep(std::time::Duration::from_millis(100));
        }
    });

    handle.join().unwrap();
    println!("");
}
