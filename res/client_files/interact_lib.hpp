#ifndef _INTERACT_LIB_HPP
#define _INTERACT_LIB_HPP
#include <iostream>
#include <stdio.h>
#include <unistd.h>
#include <stdlib.h>
#include <stdint.h>
#include <string>

//#define VERBOSE_IO

bool isBigEndian() {
    union {
        uint32_t i;
        char c[4];
    } bint = {0x01020304};

    return bint.c[0] == 1;
}

const bool IS_BIG_ENDIAN = isBigEndian();

void* advanceBytes(void* ptr, int n) {
    return (void*) (((uint8_t*) ptr) + n);
}

void readBytes(int n, void* out) {
    int numRead = 0;

    while (numRead < n) {
        int x = read(STDIN_FILENO, advanceBytes(out, numRead), n - numRead);

        if (x == -1) {
            std::cerr << "Unexpected error while reading from stdin" << std::endl;
            exit(1);
        } else if (x == 0) {
            std::cerr << "Unexpected EOF" << std::endl;
            exit(1);
        }

        numRead += x;
    }
}

void writeBytes(int n, void* bytes) {
#ifdef VERBOSE_IO
    std::cerr << "  Writing " << n << " bytes: [ ";

    for (int i = 0; i < n; i++) {
        std::cerr << +((uint8_t*) bytes)[i] << " ";
    }

    std::cerr << "]";

    std::cerr << std::endl;
#endif

    int numWritten = 0;
    while (numWritten < n) {
        int written = write(STDOUT_FILENO, advanceBytes(bytes, numWritten), n - numWritten);

        if (written == -1) {
            std::cerr << "Unexpected error while writing to stdout" << std::endl;
            exit(1);
        }

        numWritten += written;
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
    /*if (IS_BIG_ENDIAN) return;

    reverseEndinness(n, data);*/
}

void makeBigEndian(int n, void* data) {
    /*if (IS_BIG_ENDIAN) return;

    reverseEndinness(n, data);*/
}

template<typename T>
void readData(T& out) {
    constexpr size_t SIZE = sizeof(T);

#ifdef VERBOSE_IO
    std::cerr << "Trying to read " << sizeof(T) << " bytes" << std::endl;
#endif
    
    uint8_t* buffer = (uint8_t*) &out;
    readBytes(SIZE, buffer);
    makeSystemEndian(SIZE, buffer);

#ifdef VERBOSE_IO
    std::cerr << "Read " << typeid(T).name() << " [" << +out << "]" << std::endl;
#endif
}

void readString(std::string& out) {
#ifdef VERBOSE_IO
    std::cerr << "Trying to read string!" << std::endl;
#endif
    uint32_t size;
    readData(size);

#ifdef VERBOSE_IO
    std::cerr << "String size: " << size << std::endl;
#endif

    out.resize(size);

    readBytes(size, &out[0]);

#ifdef VERBOSE_IO
    std::cerr << "Read string " << out << std::endl;
#endif
}

template<typename T>
void writeData(T x) {
    constexpr size_t SIZE = sizeof(T);

#ifdef VERBOSE_IO
    std::cerr << "Writing " << typeid(T).name() << " (" << sizeof(T) << " bytes)" <<" [" << +x << "]" << std::endl;
#endif

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