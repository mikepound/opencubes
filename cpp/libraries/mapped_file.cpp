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
#include <fstream>
#include <string>

// POSIX/Linux APIs
#include <fcntl.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <unistd.h>

#include <sys/ioctl.h>
#include <linux/fs.h>

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
        // std::fprintf(stderr, "Error opening file for reading\n");
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
            // std::fprintf(stderr, "Error opening file:%s\n", std::strerror(errno));
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
            // std::fprintf(stderr, "Error opening file:%s\n", std::strerror(errno));
            return -1;
        }
        fd_rw = true;

        if(flags & FSTUNE) {
            int flags = 0;
            ioctl(fd, FS_IOC_GETFLAGS, &flags);
            flags |= FS_NOATIME_FL | FS_NOCOW_FL;
            ioctl(fd, FS_IOC_SETFLAGS, &flags);
        }
        return truncate(maxsize);

    } else if ((flags & RESIZE) != 0) {
        fd = ::open64(fname, O_RDWR | O_CLOEXEC, fperms);
        if (fd == -1) {
            // std::fprintf(stderr, "Error opening file:%s\n", std::strerror(errno));
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

region::region(std::shared_ptr<file> src, seekoff_t fpos, len_t size, len_t window) : mfile(src) {
    std::lock_guard lock(mfile->mut);
    remap(fpos, size, window);
}

region::region(std::shared_ptr<file> src) : mfile(src) {
    std::lock_guard lock(mfile->mut);
    auto sz = mfile->size();
    remap(0, sz, sz);
}

region::~region() {
    // destructor is not thread-safe.
    map_fseek = 0;
    remap(0, 0, 0);
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
void region::remap(const seekoff_t fpos, const len_t size, const len_t window) {
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
        auto newsize = roundUp(std::max(size, window));
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
        auto newsize = roundUp(std::max(size, window));
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
            // If this gets triggered we are in deep trouble
            std::fprintf(stderr, "Error memory-mapping file:%s %lu %d %lu\n", std::strerror(errno), size, mfile->fd, fpos);
            std::fprintf(stderr, "Dumping /proc/self/maps:\n");
            // for debugging information try print /proc/self/mmaps contents
            // as this explains why we hit some limit of the system.
            std::ifstream fmaps("/proc/self/maps");
            std::string buf;
            int count = 0;
            while(std::getline(fmaps, buf)) {
                std::fprintf(stderr, "%s\n", buf.c_str());
                ++count;
            }
            std::fprintf(stderr, "counted %d memory-maps in process.\n", count);



            // todo: if this really is an hard limit of the hardware
            // for *number of mmap() areas* this means we forced to:
            // - register all regions in ordered list by mapped seek offset in the mapped::file
            // - when mmap fails we have to merge adjacent regions
            // - reference count the regions
            // - data() returned memory address becomes even more unstable:
            //   it is invalidated by adjacent construction/deconstruction of region objects
            // - destruction gets complicated.
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
        map_size = roundUp(std::max(size, window));
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
    remap(fpos, usr_size, map_size);
    is_dirty = false;
}

void region::flushJump(seekoff_t fpos) {
    flush();
    std::lock_guard lock(mfile->mut);
    remap(fpos, usr_size, map_size);
}

void region::flush() {
    // only flush if dirty and RW mapped.
    std::lock_guard lock(mfile->mut);
    if (is_dirty && mfile->is_rw()) {
        is_dirty = false;
        auto flush_begin = (void*)roundDown((uintptr_t)usr_ptr);
        auto flush_len = roundUp(usr_size);
        if (flush_begin < usr_ptr) flush_len += PAGE_SIZE;
        if (msync(flush_begin, flush_len, MS_ASYNC)) {
            std::fprintf(stderr, "Error flushing memory-map:%s\n", std::strerror(errno));
        }
    }
}

void region::sync() {
    // only flush if dirty and RW mapped.
    std::lock_guard lock(mfile->mut);
    if (is_dirty && mfile->is_rw()) {
        is_dirty = false;
        auto flush_begin = (void*)roundDown((uintptr_t)usr_ptr);
        auto flush_len = roundUp(usr_size);
        if (flush_begin < usr_ptr) flush_len += PAGE_SIZE;
        if (msync(flush_begin, flush_len, MS_SYNC)) {
            std::fprintf(stderr, "Error flushing memory-map:%s\n", std::strerror(errno));
        }
    }
}

void region::writeAt(seekoff_t fpos, len_t datasize, const void* data) {
    auto srcmem = (const char*)data;

    std::lock_guard lock(mfile->mut);
    if(mfile->size() < fpos+datasize && mfile->truncate(fpos+datasize)) {
        return;
    }

    // does write fall out the mapped area begin?
    if (fpos < map_fseek) {
        // max size that can be written before map_fseek
        ssize_t wr = std::min(map_fseek - fpos, datasize);
        if (pwrite(mfile->fd, srcmem, wr, fpos) != wr) {
            std::fprintf(stderr, "Error writing file:%s\n", std::strerror(errno));
        }
        srcmem += wr;
        fpos += wr;
        datasize -= wr;
    }

    if (fpos >= map_fseek && fpos < map_fseek + map_size && datasize) {
        // max size that can be copied into this mapping:
        ssize_t wr = std::min(map_size - (fpos - map_fseek), datasize);
        std::memcpy((char*)map_ptr + (fpos - map_fseek), srcmem, wr);
        srcmem += wr;
        fpos += wr;
        datasize -= wr;
    }

    // does write fall out the mapped area end?
    if (datasize) {
        // write into backing file after the mapped area:
        if (pwrite(mfile->fd, srcmem, datasize, fpos) != ssize_t(datasize)) {
            std::fprintf(stderr, "Error writing file:%s\n", std::strerror(errno));
        }
    }
}

void region::readAt(seekoff_t fpos, len_t datasize, void* data) const {
    auto dstmem = (char*)data;

    // does read fall out the mapped area begin?
    if (fpos < map_fseek) {
        // max size that can be written before map_fseek
        ssize_t rd = std::min(map_fseek - fpos, datasize);
        if (pread(mfile->fd, dstmem, rd, fpos) != rd) {
            std::fprintf(stderr, "Error reading file:%s\n", std::strerror(errno));
        }
        dstmem += rd;
        fpos += rd;
        datasize -= rd;
    }

    if (fpos >= map_fseek && fpos < map_fseek + map_size && datasize) {
        // max size that can be copied from this mapping:
        ssize_t rd = std::min(map_size - (fpos - map_fseek), datasize);
        std::memcpy(dstmem, (char*)map_ptr + (fpos - map_fseek), rd);
        dstmem += rd;
        fpos += rd;
        datasize -= rd;
    }

    // does read fall out the mapped area end?
    if (datasize) {
        // read from backing file after the mapped area:
        if (pread(mfile->fd, dstmem, datasize, fpos) != ssize_t(datasize)) {
            std::fprintf(stderr, "Error reading file:%s\n", std::strerror(errno));
        }
    }
}


void region::resident(bool resident) {
    std::lock_guard lock(mfile->mut);
    auto _begin = (void*)roundDown((uintptr_t)usr_ptr);
    auto _len = roundUp(usr_size);
    if (_begin < usr_ptr) _len += PAGE_SIZE;

    if(madvise(_begin, _len, resident ? MADV_WILLNEED : MADV_DONTNEED)) {
            std::fprintf(stderr,"Error setting memory-map residency:%s\n",std::strerror(errno));
    }
}

/*
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
