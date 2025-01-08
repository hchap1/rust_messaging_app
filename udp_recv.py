import socket

# Define the port number and address for receiving broadcasts
PORT = 12345
BROADCAST_IP = "0.0.0.0"  # Use "0.0.0.0" to listen on all available interfaces

# Create a UDP socket
sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
sock.setsockopt(socket.SOL_SOCKET, socket.SO_BROADCAST, 1)

# Bind the socket to listen on the specified port
sock.bind((BROADCAST_IP, PORT))

print(f"Listening for UDP broadcasts on port {PORT}...")

try:
    while True:
        # Receive a broadcast message
        data, addr = sock.recvfrom(1024)  # Buffer size is 1024 bytes
        print(f"Received message from {addr}: {data.decode('utf-8')}")
except KeyboardInterrupt:
    print("\nProgram terminated by user.")
finally:
    sock.close()
