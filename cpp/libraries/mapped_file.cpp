/**
 * Copyright 2023 Jarmo A Tiitto
 *
 * Permission is hereby granted, free of charge, to any person
 * obtaining a copy of this software and associated documentation
 * files (the “Software”), to deal in the Software without
 * restriction, including without limitation the rights to use, copy,
 * modify, merge, publish, distribute, sublicense, and/or sell copies
 * of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be
 * included in all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND,
 * EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
 * MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
 * NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS
 * BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN
 * ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
 * CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */
#include "mapped_file.hpp"

#include <algorithm>
#include <cstring>
#include <iostream>
#include <string>

// POSIX/Linux APIs
#include <fcntl.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <unistd.h>

#ifndef MAP_HUGE_2MB
#define MAP_HUGE_2MB (21 << MAP_HUGE_SHIFT)
#define MAP_HUGE_1GB (30 << MAP_HUGE_SHIFT)
#endif

namespace mapped {

/**
 * Mapped file POSIX/Linux compatible implementation
 */
file::file() : fd(-1), fd_size(0) {}

file::~file() { close(); }

void file::close() {
    if (fd >= 0) {
        ::fsync(fd);
        ::close(fd);
        fd = -1;
        fd_size = 0;
    }
}

int file::open(const char* fname) {
    close();

    fd = ::open64(fname, O_RDONLY);
    if (fd == -1) {
        //std::fprintf(stderr, "Error opening file for reading\n");
        return -1;
    }

    struct stat64 finfo;
    if (fstat64(fd, &finfo)) {
        std::fprintf(stderr, "Error getting file size: %s\n", std::strerror(errno));
        return -1;
    }
    fd_size = finfo.st_size;
    fd_rw = false;
    return 0;
}

int file::openrw(const char* fname, size_t maxsize, int flags) {
    // create new files with "normal" permissions: "-rw-r--r--"
    const mode_t fperms = S_IRUSR | S_IWUSR | S_IRGRP | S_IWGRP | S_IROTH;

    close();

    maxsize = roundUp(maxsize);

    if (!flags) {
        fd = ::open64(fname, O_RDWR | O_CLOEXEC);
        if (fd == -1) {
            //std::fprintf(stderr, "Error opening file:%s\n", std::strerror(errno));
            return -1;
        }

        fd_rw = true;

        struct stat64 finfo;
        if (fstat64(fd, &finfo)) {
            std::fprintf(stderr, "Error getting file size:%s\n", std::strerror(errno));
            return -1;
        }
        return truncate(finfo.st_size);

    } else if ((flags & (CREATE | RESIZE)) == (CREATE | RESIZE)) {
        fd = ::open64(fname, O_CREAT | O_RDWR | O_TRUNC | O_CLOEXEC, fperms);
        if (fd == -1) {
            //std::fprintf(stderr, "Error opening file:%s\n", std::strerror(errno));
            return -1;
        }
        fd_rw = true;
        return truncate(maxsize);

    } else if ((flags & RESIZE) != 0) {
        fd = ::open64(fname, O_RDWR | O_CLOEXEC, fperms);
        if (fd == -1) {
            //std::fprintf(stderr, "Error opening file:%s\n", std::strerror(errno));
            return -1;
        }
        fd_rw = true;
        return truncate(maxsize);
    } else {
        std::fprintf(stderr, "Invalid open flags:%s\n", std::strerror(errno));
        return -1;
    }
}

bool file::is_rw() const { return fd_rw; }

seekoff_t file::size() const { return fd_size; }

int file::truncate(seekoff_t newsize) {
    // resize the backing file
    if (newsize != fd_size && ftruncate64(fd, newsize)) {
        std::fprintf(stderr, "Error resizing backing file:%s\n", std::strerror(errno));
        return -1;
    }
    fd_size = newsize;
    return 0;
}

/**
 * Mapped region POSIX/Linux compatible implementation.
 */

region::region(std::shared_ptr<file> src, seekoff_t fpos, len_t size) : mfile(src) {
    std::lock_guard lock(mfile->mut);
    remap(fpos, size);
}

region::region(std::shared_ptr<file> src) : mfile(src) {
    std::lock_guard lock(mfile->mut);
    remap(0, mfile->size());
}

region::~region() {
    std::lock_guard lock(mfile->mut);
    map_fseek = 0;
    remap(0, 0);
}

/**
 * This is the core implementation of mapped_file:
 * remap(0,0) releases the mapping.
 * remap(0, n) mmap roundUp(n) bytes at offset 0
 * remap(0, k) mremap roundUp(n) bytes at offset 0 (grows the existing mapping)
 * remap(n, j) munmap old region, mmap new at offset roundDown(n)
 *
 * In read-write mode the backing file is grown to fit the mapping.
 */
void region::remap(const seekoff_t fpos, const len_t size) {
    if (fpos == usr_fseek && size == usr_size) return;  // No-op
    // check if [fpos, fpos+size] fits into the existing
    // mmap() window and only adjust the user region.
    if (size && map_ptr && (map_fseek <= fpos && fpos + size <= map_fseek + map_size)) {
        usr_fseek = fpos;
        usr_ptr = (uint8_t*)map_ptr + (fpos - map_fseek);
        usr_size = size;
        return;
    }

    // if size == 0 or the usr_fseek != fpos,
    // we have to unmap the old region first, if any.
    if (!!map_ptr && (size == 0 || usr_fseek != fpos)) {
        if (::munmap(map_ptr, map_size) == -1) {
            std::fprintf(stderr, "Error mapping file memory\n");
            return;
        }
        map_ptr = nullptr;
        map_size = 0;
        usr_ptr = nullptr;
        usr_size = 0;
        if (size == 0) return;
    }
    // keep what user tried to ask:
    usr_fseek = fpos;
    usr_size = size;

    if (map_ptr && map_fseek == fpos) {
        // this mapping exists already at same map_fseek
        // remap it to grow the region.
        auto newsize = roundUp(size);
        void* newptr = mremap(map_ptr, map_size, newsize, MREMAP_MAYMOVE);
        if (newptr == MAP_FAILED) {
            std::fprintf(stderr, "Error resizing memory-map of file:%s\n", std::strerror(errno));
            std::abort();
            return;
        }
        map_ptr = newptr;
        map_size = newsize;
        return;
    }

    // create new mapping
    if (mfile->is_rw()) {
        // RW mapping
        auto newsize = roundUp(size);
        if (mfile->size() < fpos + newsize && mfile->truncate(fpos + newsize)) {
            // failed. Disk full?
            std::abort();
            return;
        }
        // mmap requires fpos && size to be multiple of PAGE_SIZE
        map_fseek = roundDown(fpos);
        if (map_fseek < fpos) {
            // adjust size to cover.
            newsize += PAGE_SIZE;
        }
        map_size = newsize;
        map_ptr = mmap(0, map_size, PROT_READ | PROT_WRITE, MAP_SHARED, mfile->fd, map_fseek);
        if (map_ptr == MAP_FAILED) {
            std::fprintf(stderr, "Error memory-mapping file:%s %lu %d %lu\n", std::strerror(errno), size, mfile->fd, fpos);
            std::abort();
            return;
        }
    } else {
        // RO mapping
        if (mfile->size() <= fpos) {
            // can't: the backing file is too small.
            std::fprintf(stderr, "Error seeking past end of file.\n");
            std::abort();
            return;
        }
        map_size = roundUp(size);
        map_fseek = roundDown(fpos);
        // Map the region. (use huge pages, don't reserve backing store)
        map_ptr = mmap(0, map_size, PROT_READ, MAP_SHARED | MAP_NORESERVE | MAP_HUGE_2MB, mfile->fd, map_fseek);

        if (!map_ptr || map_ptr == MAP_FAILED) {
            std::fprintf(stderr, "Error mapping file\n");
            std::abort();
            return;
        }
    }
    // adjust the usr_ptr to fix
    // any page misalignment.
    usr_ptr = (uint8_t*)map_ptr + (fpos - map_fseek);
}

void region::jump(seekoff_t fpos) {
    std::lock_guard lock(mfile->mut);
    remap(fpos, map_size);
    is_dirty = false;
}

void region::flushJump(seekoff_t fpos) {
    flush();
    std::lock_guard lock(mfile->mut);
    remap(fpos, map_size);
}

void region::flush() {
    // only flush if dirty and RW mapped.
    std::lock_guard lock(mfile->mut);
    if (is_dirty && mfile->is_rw()) {
        is_dirty = false;
        if (msync(map_ptr, map_size, MS_ASYNC)) {
            std::fprintf(stderr, "Error flushing memory-map:%s\n", std::strerror(errno));
        }
    }
}

void region::sync() {
    // only flush if dirty and RW mapped.
    std::lock_guard lock(mfile->mut);
    if (is_dirty && mfile->is_rw()) {
        is_dirty = false;
        if (msync(map_ptr, map_size, MS_SYNC)) {
            std::fprintf(stderr, "Error flushing memory-map:%s\n", std::strerror(errno));
        }
    }
}

/*
TODO:
void region::resident(void * paddr, size_t lenght, bool resident) {
        // Align paddr to PAGE_SIZE
        void * start = reinterpret_cast<void*>(uintptr_t(paddr) & ~(PAGE_SIZE-1));
        lenght = roundToPage(lenght);

        if(madvise(start, lenght, resident ? MADV_WILLNEED : MADV_DONTNEED)) {
                std::fprintf(stderr,"Error setting memory-map residency:%s\n",std::strerror(errno));
        }
}

void region::discard(void * paddr, size_t lenght) {
        // get range of pages that may be discarded.
        // this is always an subset of [paddr, paddr+lenght] range.
        void * start = (void*)roundUp((uintptr_t)paddr, PAGE_SIZE);
        lenght = roundDown(lenght, PAGE_SIZE);

        if(start < (char*)paddr + lenght && lenght >= PAGE_SIZE) {
                // note: errors are ignored here.
                madvise(start, lenght, MADV_REMOVE);
        }
}
*/

};  // namespace mapped
