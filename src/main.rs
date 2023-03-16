#![allow(unused_labels)]

use rand::prelude::*;
use rodio::source::{SineWave, Source};
use rodio::{OutputStream, Sink};
use std::io::{stdin, stdout, Write};
use std::str::FromStr;
use std::thread::sleep;
use std::time::Duration;

fn note_freq(note: i32) -> f32 {
    440.0 * 2_f32.powf((note - 9) as f32 / 12.0)
}

fn append_note(sink: &Sink, note: i32, duration: Duration) {
    let mut wave = SineWave::new(note_freq(note)).take_duration(duration);
    wave.set_filter_fadeout();
    sink.append(wave.amplify(0.1));
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

fn choose_biased(rng: &mut impl Rng, notes: &[i32], stats: &[i32]) -> (usize, i32) {
    let base_weight = notes.iter().max().unwrap() + 3;
    let weights = stats
        .iter()
        .map(|stat| base_weight - stat)
        .collect::<Vec<_>>();
    let weight_range = weights.iter().sum::<i32>();
    let mut num = rng.gen_range(0..weight_range);
    for ((i, note), weight) in notes.iter().enumerate().zip(weights) {
        if num < weight {
            return (i, *note);
        }
        num -= weight;
    }
    unreachable!();
}

#[test]
fn test_choose() {
    let mut rng = thread_rng();
    let notes = from_scale(0b_101011010101);
    let stats = [0, 12, 12, 12, 12, 12, 12];
    for _ in 0..4 {
        for _ in 0..16 {
            print!("{:?}", choose_biased(&mut rng, &notes, &stats).0);
        }
        println!();
    }
}

fn main() {
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();

    let mut notes = from_scale(0b_1010_1101_0101);
    notes.push(12); // literally the only reason notes is mut

    let mut rng = thread_rng();
    let mut streak = 0;
    let normal_speed = Duration::from_secs_f32(0.25);
    let slow_speed = normal_speed * 2;
    let fast_speed = normal_speed / 2;

    let mut speed = normal_speed;

    let mut stats = vec![0; notes.len()];

    play_scale(&sink, &notes, speed);
    'game_loop: loop {
        let (i, note) = choose_biased(&mut rng, &notes, &stats);
        println!("Stats: {stats:?}");
        println!("Score: {}", stats.iter().copied().sum::<i32>());
        println!("Choosing note...");
        sleep(slow_speed);
        play(&sink, note, speed);

        'guess_loop: loop {
            let Some(guess) = input_try::<i32>("Guess the note.", "> ", "?") else {
                play(&sink, note, slow_speed);
                continue;
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
                stats[i] += 1;
                streak = streak.max(0) + 1;

                println!("Correct! Streak: {streak}");
                play(&sink, 0, fast_speed);
                play(&sink, 4, fast_speed);
                play(&sink, 12, normal_speed);
                sleep(normal_speed);

                speed = normal_speed;

                println!();
                println!("{}", "-".repeat(32));
                break 'guess_loop;
            } else {
                stats[i] -= 1;
                streak = streak.min(1) - 1;
                println!("Incorrect :P Streak: {streak}");
                play(&sink, 3, fast_speed);
                play(&sink, 2, fast_speed);
                sleep(normal_speed);

                if streak <= -3 {
                    speed = slow_speed;
                }
                if streak <= -1 {
                    play_scale(&sink, &notes, speed);
                }
                println!();
            }
        }
    }
}
