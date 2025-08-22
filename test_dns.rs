use std::net::UdpSocket;
use std::time::Duration;

fn main() -> std::io::Result<()> {
    // Create a simple DNS query for seed.kaspa.org A record
    let query = [
        0x12, 0x34, // ID
        0x01, 0x00, // Flags: standard query
        0x00, 0x01, // Questions: 1
        0x00, 0x00, // Answer RRs: 0
        0x00, 0x00, // Authority RRs: 0
        0x00, 0x00, // Additional RRs: 0
        // Question section
        0x04, 0x73, 0x65, 0x65, 0x64, // "seed"
        0x05, 0x6b, 0x61, 0x73, 0x70, 0x61, // "kaspa"
        0x03, 0x6f, 0x72, 0x67, // "org"
        0x00, // End of name
        0x00, 0x01, // Type: A
        0x00, 0x01, // Class: IN
    ];

    println!("Sending DNS query to 127.0.0.1:5354");
    
    // Create socket
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_read_timeout(Some(Duration::from_secs(5)))?;
    
    // Send query
    socket.send_to(&query, "127.0.0.1:5354")?;
    println!("Query sent, waiting for response...");
    
    // Receive response
    let mut buffer = [0u8; 512];
    match socket.recv_from(&mut buffer) {
        Ok((len, addr)) => {
            println!("Received {} bytes from {}", len, addr);
            println!("Response: {:?}", &buffer[..len]);
        }
        Err(e) => {
            println!("Error receiving response: {}", e);
        }
    }
    
    Ok(())
}
