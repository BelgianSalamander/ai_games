#ifndef _INTERACT_LIB_HPP
#define _INTERACT_LIN_HPP
#include <iostream>
#include <stdio.h>
#include <unistd.h>
#include <stdlib.h>
#include <stdint.h>
#include <string>

bool isBigEndian() {
    union {
        uint32_t i;
        char c[4];
    } bint = {0x01020304};

    return bint.c[0] == 1;
}

const bool IS_BIG_ENDIAN = isBigEndian();

void readBytes(int n, void* out) {
    int x = read(STDIN_FILENO, out, n);

    if (x == -1) {
        std::cerr << "Unexpected error while reading from stdin" << std::endl;
        exit(1);
    } else if (x == 0) {
        std::cerr << "Unexpected EOF" << std::endl;
        exit(1);
    }
}

void writeBytes(int n, void* bytes) {
    int numWritten = 0;
    while (numWritten < n) {
        int written = write(STDOUT_FILENO, bytes + numWritten, n - numWritten);

        if (written == -1) {
            std::cerr << "Unexpected error while writing to stdout" << std::endl;
            exit(1);
        }

        numWritten -= written;
    }
}

void reverseEndinness(int n, void* data) {
    uint8_t* start = (uint8_t*) data;
    uint8_t* end = start + n - 1;

    while (start < end) {
        uint8_t temp = *start;
        *start = *end;
        *end = temp;

        start++;
        end--;
    }
}

void makeSystemEndian(int n, void* data) {
    if (IS_BIG_ENDIAN) return;

    reverseEndinness(n, data);
}

void makeBigEndian(int n, void* data) {
    if (IS_BIG_ENDIAN) return;

    reverseEndinness(n, data);
}

template<typename T>
void readData(T& out) {
    constexpr size_t SIZE = sizeof(T);
    
    uint8_t* buffer = (uint8_t*) &out;
    readBytes(SIZE, buffer);
    makeSystemEndian(SIZE, buffer);
}

void readString(std::string& out) {
    uint32_t size;
    readData(size);

    out.resize(size);

    readBytes(size, &out[0]);
}

template<typename T>
void writeData(T x) {
    constexpr size_t SIZE = sizeof(T);

    void* bytes = (void*) &x;
    makeBigEndian(SIZE, bytes);

    writeBytes(SIZE, bytes);
}

void writeString(std::string& s) {
    writeData<uint32_t>(s.length());
    writeBytes(s.length(), (void*) &s[0]);
}

void flushStreams() {
    fflush(stdout);
}

#endif