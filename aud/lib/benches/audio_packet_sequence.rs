use aud_lib::comms::*;
use criterion::{black_box, criterion_group, criterion_main, Criterion}; // replace `your_crate` with your actual crate name
use rand::Rng;

fn gen_linear_packets(n: usize) -> Vec<AudioPacket> {
    (0..n)
        .map(|i| AudioPacket::new(i as u64, &[0., 1., 2., 3., 4.], 1))
        .collect()
}

fn gen_semi_ordered_packets(n: usize) -> Vec<AudioPacket> {
    let mut sequence: Vec<u64> = (0..n as u64).collect();

    for i in (1..n as usize).step_by(2) {
        if i + 1 < n {
            sequence.swap(i, i + 1);
        }
    }

    sequence
        .into_iter()
        .map(|i| AudioPacket::new(i as u64, &[0., 1., 2., 3., 4.], 1))
        .collect()
}

fn bench_push_packet(c: &mut Criterion) {
    let mut seq = AudioPacketSequence::default();
    let mut rng = rand::thread_rng();
    let packet = AudioPacket::new(0, &[0., 1., 2., 3., 4.], 1);

    c.bench_function("push a packet", |b| {
        b.iter(|| {
            let mut packet = packet.clone();
            packet.header.index = rng.gen();
            seq.push(black_box(packet));
        })
    });
}

fn bench_consuming_sequence(c: &mut Criterion) {
    let mut seq = AudioPacketSequence::default();
    let packet = AudioPacket::new(0, &[0., 1., 2., 3., 4.], 1);
    const NUM_PACKETS: usize = 1_000;
    for i in 0..NUM_PACKETS {
        let mut packet = packet.clone();
        packet.header.index = i as u64;
        seq.push(packet);
    }

    c.bench_function(&format!("consume {NUM_PACKETS} packets"), |b| {
        b.iter(|| {
            seq.consume();
        })
    });
}

fn bench_parse_ordered_packets(c: &mut Criterion) {
    for num_packets in [4, 64, 1024] {
        let packets = gen_linear_packets(num_packets);
        c.bench_function(
            &format!("parse ordered packet sequence : {num_packets}"),
            |b| {
                b.iter(|| {
                    let result =
                        AudioPacketSequence::with_packets(black_box(packets.clone())).consume();
                    result
                })
            },
        );
    }
}

fn bench_parse_semi_ordered_packets(c: &mut Criterion) {
    for num_packets in [4, 64, 1024] {
        let packets = gen_semi_ordered_packets(num_packets);
        c.bench_function(
            &format!("parse semi-ordered packet sequence : {num_packets}"),
            |b| {
                b.iter(|| {
                    let result =
                        AudioPacketSequence::with_packets(black_box(packets.clone())).consume();
                    result
                })
            },
        );
    }
}

criterion_group!(
    audio_packet_seq,
    bench_push_packet,
    bench_consuming_sequence,
    bench_parse_ordered_packets,
    bench_parse_semi_ordered_packets
);
criterion_main!(audio_packet_seq);
