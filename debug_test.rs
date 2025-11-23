use std::fs;

fn main() {
    let data = fs::read("tests/test-data/test.rvm").expect("Failed to read file");
    
    println!("File size: {} bytes", data.len());
    println!("First 100 bytes:");
    for (i, byte) in data.iter().take(100).enumerate() {
        if i % 16 == 0 {
            print!("\n{:04x}: ", i);
        }
        print!("{:02x} ", byte);
    }
    println!("\n");
    
    // Try to decode first chunk type
    if data.len() >= 16 {
        let chunk_bytes = &data[0..16];
        println!("First 16 bytes as UTF-32 BE:");
        for i in 0..4 {
            let offset = i * 4;
            let ch = u32::from_be_bytes([
                chunk_bytes[offset],
                chunk_bytes[offset + 1],
                chunk_bytes[offset + 2],
                chunk_bytes[offset + 3],
            ]);
            if let Some(c) = char::from_u32(ch) {
                print!("{}", c);
            }
        }
        println!();
    }
}
