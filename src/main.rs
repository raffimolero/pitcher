#![allow(unused_labels)]

use std::{
    fmt::Display,
    io::{stdin, stdout, Write},
    str::FromStr,
    thread::sleep,
    time::Duration,
};

use rand::prelude::*;
use rodio::{
    source::{SineWave, Source},
    {OutputStream, Sink},
};

fn note_freq(note: i32) -> f32 {
    440.0 * 2_f32.powf((note - 9) as f32 / 12.0)
}

fn append_note(sink: &Sink, note: i32, duration: Duration) {
    let mut wave = SineWave::new(note_freq(note)).take_duration(duration);
    wave.set_filter_fadeout();
    sink.append(wave.amplify(0.5));
}

fn from_scale(bits: u16) -> Vec<i32> {
    let mut mask = 1 << 11;
    let mut v = vec![];
    for i in 0..12 {
        if bits & mask != 0 {
            v.push(i)
        }
        mask >>= 1;
    }
    v
}

fn input_line(prompt: &str) -> String {
    print!("{prompt}");
    stdout().flush().unwrap();

    let mut buf = String::new();
    stdin().read_line(&mut buf).unwrap();
    buf
}

fn input_try<T: FromStr>(msg: &str, prompt: &str, cancel: &str) -> Option<T> {
    if !msg.is_empty() {
        println!("{msg}");
    }
    loop {
        let input = input_line(prompt);
        let input = input.trim();
        if input == cancel {
            return None;
        }
        match input.parse() {
            Ok(out) => return Some(out),
            Err(_) => println!("[Bad input. Try again.]"),
        }
    }
}

fn play(sink: &Sink, note: i32, duration: Duration) {
    append_note(sink, note, duration);
    sink.sleep_until_end();
}

fn play_scale(sink: &Sink, notes: &[i32], note_duration: Duration) {
    for note in notes {
        print!("{note} ");
        stdout().flush().unwrap();
        play(sink, *note, note_duration);
    }
    println!();
}

/// panics if weights is empty or is longer than items
fn choose_biased<'a, T>(rng: &mut impl Rng, items: &'a [T], weights: &[f32]) -> (usize, &'a T) {
    let weight_range = weights.iter().sum::<f32>();
    let mut num = rng.gen_range(0.0..weight_range);
    for (item, &weight) in items.iter().enumerate().zip(weights) {
        if num < weight {
            return item;
        }
        num -= weight;
    }
    panic!();
}

#[derive(Debug, Clone, Copy, Default)]
struct Stat {
    wins: u32,
    losses: u32,
}

impl Stat {
    fn total(&self) -> u32 {
        self.wins + self.losses
    }

    fn rate(&self) -> f32 {
        let total = self.total();
        if total == 0 {
            0.0
        } else {
            self.wins as f32 / total as f32
        }
    }

    fn weight(&self) -> f32 {
        1.25 - self.rate()
    }
}

impl Display for Stat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:>4}:{:<4} |   {:>3.0}%",
            self.wins,
            self.losses,
            self.rate() * 100.0,
        )
    }
}

#[derive(Debug, Clone, Default)]
struct Stats(Vec<Stat>);

impl Stats {
    fn win(&mut self, index: usize) {
        self.0[index].wins += 1;
    }

    fn lose(&mut self, index: usize) {
        self.0[index].losses += 1;
    }

    fn weights(&self) -> Vec<f32> {
        self.0.iter().map(|stat| stat.weight()).collect()
    }
}

impl Display for Stats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "|-------|-----------|----------|-------------|")?;
        writeln!(f, "|  note |  win:loss |   win%   | pick weight |")?;
        let mut total = Stat::default();
        for (i, stat) in self.0.iter().enumerate() {
            total.wins += stat.wins;
            total.losses += stat.losses;
            writeln!(f, "|  {i:>2}   | {stat}   |    {:>1.3}    |", stat.weight())?;
        }
        writeln!(f, "|-------|-----------|----------|-------------|")?;
        writeln!(f, "| total | {total}   |")?;
        writeln!(f, "|-------|-----------|----------|")
    }
}

fn main() {
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();

    let mut notes = from_scale(0b_1111_1111_1111);
    notes.push(12); // literally the only reason notes is mut

    let mut rng = thread_rng();
    let mut streak = 0;
    let normal_speed = Duration::from_secs_f32(0.2);
    let slow_speed = normal_speed * 2;
    let fast_speed = normal_speed / 2;

    let mut speed = normal_speed;

    let mut stats = Stats(vec![Stat::default(); notes.len()]);

    play_scale(&sink, &notes, speed);
    'game_loop: loop {
        let (i, &note) = choose_biased(&mut rng, &notes, &stats.weights());
        println!("Stats:\n{stats}");
        println!("Choosing note...");
        sleep(slow_speed);
        play(&sink, note, speed);

        'guess_loop: loop {
            let Some(guess) = input_try::<i32>("Guess the note.", "> ", "?") else {
                play(&sink, note, slow_speed);
                continue 'guess_loop;
            };

            println!("You played: {guess}");
            sleep(fast_speed);
            play(&sink, guess, speed);
            sleep(fast_speed);
            println!("Correct was:");
            sleep(fast_speed);
            play(&sink, note, speed);
            sleep(fast_speed);

            if guess == note {
                stats.win(i);
                streak = streak.max(0) + 1;

                println!("Correct! Streak: {streak}");
                play(&sink, 0, fast_speed);
                play(&sink, 4, fast_speed);
                play(&sink, 12, normal_speed);
                sleep(normal_speed);

                speed = normal_speed;

                println!();
                break 'guess_loop;
            } else {
                stats.lose(i);
                streak = streak.min(1) - 1;
                println!("Incorrect :P Streak: {streak}");
                play(&sink, 3, fast_speed);
                play(&sink, 2, fast_speed);
                sleep(normal_speed);
                println!();
            }
        }
    }
}
