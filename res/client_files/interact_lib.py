import sys
import struct

READ_BUF = sys.stdin.buffer
WRITE_BUF = sys.stdout.buffer

def read(n):
    res = READ_BUF.read(n)
    while len(res) < n:
        res += READ_BUF.read(n - len(res))
        
    return res

def read_u8():
    return struct.unpack('<B', read(1))[0]

def read_u16():
    return struct.unpack('<H', read(2))[0]

def read_u32():
    return struct.unpack('<I', read(4))[0]

def read_u64():
    return struct.unpack('<Q', read(8))[0]

def read_i8():
    return struct.unpack('<b', read(1))[0]

def read_i16():
    return struct.unpack('<h', read(2))[0]

def read_i32():
    return struct.unpack('<i', read(4))[0]

def read_i64():
    return struct.unpack('<q', read(8))[0]

def read_f32():
    return struct.unpack('<f', read(4))[0]

def read_f64():
    return struct.unpack('<d', read(8))[0]

def read_bool():
    return read_u8() != 0

def read_str():
    length = read_u32()
    return read(length).decode('utf-8')

def write_u8(val):
    WRITE_BUF.write(struct.pack('<B', val))
    
def write_u16(val):
    WRITE_BUF.write(struct.pack('<H', val))
    
def write_u32(val):
    WRITE_BUF.write(struct.pack('<I', val))
    
def write_u64(val):
    WRITE_BUF.write(struct.pack('<Q', val))
    
def write_i8(val):
    WRITE_BUF.write(struct.pack('<b', val))
    
def write_i16(val):
    WRITE_BUF.write(struct.pack('<h', val))
    
def write_i32(val):
    WRITE_BUF.write(struct.pack('<i', val))
    
def write_i64(val):
    WRITE_BUF.write(struct.pack('<q', val))
    
def write_f32(val):
    WRITE_BUF.write(struct.pack('<f', val))
    
def write_f64(val):
    WRITE_BUF.write(struct.pack('<d', val))
    
def write_bool(val):
    WRITE_BUF.write(struct.pack('<B', 1 if val else 0))
    
def write_str(val):
    data = val.encode('utf-8')
    write_u32(len(data))
    WRITE_BUF.write(data)
    
def flush():
    WRITE_BUF.flush()
    
sys.stderr.write('Interact library loaded\n')