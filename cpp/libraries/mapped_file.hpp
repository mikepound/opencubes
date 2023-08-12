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
#ifndef MAPPEDFILE_HPP_INCLUDED
#define MAPPEDFILE_HPP_INCLUDED

#include <algorithm>
#include <cassert>
#include <cstddef>
#include <cstdint>
#include <memory>
#include <mutex>
#include <type_traits>

/**
 * Memory mapped file I/O utilities
 * - mapped::file class for opening an file
 * - mapped::region class for RW/RO memory mapping part the file instance.
 * - mapped::struct_region<T> template for RW/RO accessing part the file as specified type.
 * - mapped::array_region<T> template for RW/RO accessing part of the file as array of T elements.
 *
 * @note
 *  When doing read-only mapping the region instance
 *  should be const qualified as this restricts
 *  the region class API to read-only operations and prevents
 *  accidental modification of the file.
 *  Use std::make_unique<const region>(<args>) in this case.
 *
 * @note
 *  When using the read-write features the backing file is resized
 *  in multiple PAGE_SIZE blocks even if the actually mapped size is
 *  something else.
 *  openrw(...,size,RESIZE) always truncates the file to roundUp(size).
 *  You should do file->truncate(< sizeInBytes>) to make the file
 *  size exactly what you want before the file is closed.
 *
 *  Modified regions should flush() or sync() before they are destroyed
 *  or the modified data may not end up in the file.
 *
 * TODO:
 *  - Two region instances should not overlap,
 *    i.e same portion of the file should not be mapped twice.
 *    (Not sure if this is actually broken now, but you have been warned)
 * -  Multi-threading support not tested/written.
 *    Currently the same mapped region can be used by multiple threads,
 *    but cannot it be modified.
 * -  Better error handling. (exceptions?, error codes?)
 *    Currently critical errors are printed and std::abort() is called.
 *    How do we handle system errors that happen in constructors?
 */
namespace mapped {

const size_t PAGE_SIZE = 4096;

static inline size_t roundToPage(ptrdiff_t x) { return (std::max<ptrdiff_t>(0, x - 1) & ~(PAGE_SIZE - 1)) + PAGE_SIZE; }

constexpr inline size_t roundUp(uintptr_t x) { return (x + (PAGE_SIZE - 1)) & ~(PAGE_SIZE - 1); }

constexpr inline size_t roundDown(uintptr_t x) { return (x & ~(PAGE_SIZE - 1)); }

/**
 * seekoff_t: Position of the file cursor
 */
using seekoff_t = uint64_t;
/**
 * len_t: length of file data
 */
using len_t = size_t;

class file;

/**
 * Memory-mapped region
 * @brief
 * the base class memory-maps an raw range of bytes from the backing file.
 */
class region {
   protected:
    std::mutex mtx;
    // actually mapped region:
    void* map_ptr = nullptr;
    size_t map_size = 0;
    seekoff_t map_fseek = 0;
    // what constructor asked:
    void* usr_ptr = nullptr;
    size_t usr_size = 0;
    seekoff_t usr_fseek = 0;
    // todo: maybe use std::weak_ptr?
    // that would allow file to be released and
    // any any existing region(s) would still work.
    // (but only if remap() is not called)
    std::shared_ptr<file> mfile;
    // non-const data access sets is_dirty.
    bool is_dirty = false;

    void remap(const seekoff_t fpos, const len_t size, const len_t window);

    region() {}

   public:
    /**
     * Open memory mapped region into a file.
     * @brief
     * Seeks at fpos in file and map size bytes
     * starting from that position in file.
     * @param window
     *  over-extend mapping up to max(size,window) bytes.
     *  Setting window bigger than size allows more efficient operation:
     *  [fpos, fpos + window] area is memory mapped
     *  but region will only operate on the
     *  [roundDown(fpos), roundup(fpos+size)]
     *  sub-portion of the memory.
     * @note
     * - Seeking past the EOF in file that is read-only will fail.
     *   The mapped size may extend past EOF but accessing past EOF
     *   either returns undefined data or program is terminated by OS.
     *   (EOF is at file->size())
     * - Seeking past the EOF that is read-write
     *   grows the backing file to fit the mapping.
     *   The backing file is always extended in multiple of PAGE_SIZE bytes.
     * @note
     *  If size and/or fpos are not aligned to multiple of PAGE_SIZE
     *  they are forcibly aligned internally. This results in
     *  regionSize() and regionSeek() that may differ compared to
     *  size() and getSeek().
     *  Side-effect is that backing file may grow more than expected.
     */
    region(std::shared_ptr<file> src, seekoff_t fpos, len_t size, len_t window = 0);

    /**
     * Open memory mapped region into the file
     * @brief
     *  same as region(myfile, 0, myfile.size())
     *  and memory maps the entire file.
     */
    explicit region(std::shared_ptr<file> src);

    /**
     * Note: even if region was modified,
     * destructor will not flush()/sync() before tearing down the mapping.
     */
    virtual ~region();

    // region is not copyable
    region(const region&) =delete;
    region& operator=(const region&) =delete;

    // region is moveable
    friend void swap(region& a, region& b) {
        using std::swap;
        //std::lock(a.mtx,b.mtx);
        //std::lock_guard l0(a.mtx, std::adopt_lock);
        //std::lock_guard l1(b.mtx, std::adopt_lock);
        swap(a.map_ptr,b.map_ptr);
        swap(a.map_size,b.map_size);
        swap(a.map_fseek,b.map_fseek);
        swap(a.usr_ptr,b.usr_ptr);
        swap(a.usr_size,b.usr_size);
        swap(a.usr_fseek,b.usr_fseek);
        swap(a.mfile,b.mfile);
        swap(a.is_dirty,b.is_dirty);
    }
    region(region&& mv) : region() {
        swap(*this, mv);
    }
    region& operator=(region&& mv) {
        swap(*this, mv);
        return *this;
    }

    /**
     * Get data pointer.
     */
    const void* data() const { return usr_ptr; }
    void* data() {
        is_dirty = true;
        return usr_ptr;
    }

    std::shared_ptr<file> getFile() { return mfile; }

    /**
     * Get the seek used to init this region.
     */
    seekoff_t getSeek() const { return usr_fseek; }
    /**
     * Get the size used to init this region.
     */
    len_t size() const { return usr_size; }

    /**
     * Get page aligned seek <= getSeek()
     */
    seekoff_t regionSeek() const { return map_fseek; }
    /**
     * Get page aligned size >= size()
     */
    len_t regionSize() const { return map_size; }

    /**
     * Resize the mapped region.
     * @note the mapped memory address may move,
     * but current contents are preserved.
     * @warn all pointers or references into
     * the mapping are invalidated.
     */
    void resize(len_t newsize);

    /**
     * @brief over-extend mapping up to max(size(),window) bytes.
     *  Setting window bigger than size() allows more efficient operation:
     *  [regionSeek(), regionSeek() + window] area is memory mapped
     *  but region will only operate on the
     *  [roundDown(getSeek()), roundUp(getSeek()+size())]
     *  sub-portion of the memory.
     */
    void window(len_t window = 0);

    /**
     * Flush mapped memory region into the file.
     * @brief this is an hint to operating system that
     * memory region shall be synchronized to disk.
     * It may not wait for this to have completed before returning.
     * @note only the page aligned region
     * [roundDown(data()), roundUp(data()+size())]
     * is flushed.
     * @note Use sync() instead if you must guarantee the data has
     * reached persistent storage.
     */
    void flush();

    /**
     * Synchronize modified memory region onto disk.
     */
    void sync();

    /**
     * Write data into the backing file.
     * @brief
     *  writeAt() stores range of bytes into the backing file.
     * @note
     *  The region doesn't need to have this area to be memory-mapped:
     *  The data that falls into the memory-mapped
     *  [regionSeek(), regionSeek()+regionSize()] area is simply memcpy'ed.
     *  Any data that falls out this window is written directly
     *  into the backing file.
     *  The backing file is grown to fit the data when needed.
     */
    void writeAt(seekoff_t fpos, len_t datasize, const void* data);

    /**
     * Read data from the backing file.
     * @brief
     *  readAt() reads [fpos, fpos+datasize] range of bytes from the backing file
     * @note
     *  The region doesn't need to have this area memory-mapped
     *  The read out area that falls into the memory-mapped
     *  [regionSeek(), regionSeek()+regionSize()] area is simply memcpy'ed.
     *  Any data that falls out this window is read directly
     *  from the backing file.
     */
    void readAt(seekoff_t fpos, len_t datasize, void* data) const;

    /**
     * Set memory region to resident/or released.
     * @brief setting memory range to non-resident state
     * causes system to drop the data from system memory.
     * Reading non-resident memory region again causes system to
     * fetch data from the disk again.
     * @warn if memory region is not flushed before setting
     * resident(false) any writes may be discarded to backing file.
     */
    void resident(bool state);

    /**
     * Discard memory region.
     * @brief discarding memory range causes system
     * to reclaim the memory *and* the on-disk area.
     * This means the data is lost in the mapped memory region,
     * and any data within will not be written onto disk by sync()
     * Subsequent reads after discard() return zero filled data.
     * @note
     *  The discarded area shall be within the mapped area.
     * @param fpos
     *  file offset from begin of this mapping. (getSeek() + fpos)
     * @param datasize
     *  length of the data area to discard.
     */
    void discard(seekoff_t fpos, len_t datasize);

    /**
     * Seek in the file to fpos position and
     * remap the memory region there.
     * @warn all pointers or references into
     * the mapping are invalidated.
     */
    void jump(seekoff_t fpos);

    /**
     * Flush the current region and
     * Seek in the file to fpos position and
     * remap the memory region there.
     * @warn all pointers or references into
     * the mapping are invalidated.
     */
    void flushJump(seekoff_t fpos);
};

static_assert(std::is_move_constructible_v<region>);
static_assert(std::is_move_assignable_v<region>);
static_assert(std::is_swappable_v<region>);

/**
 * Typed region.
 * struct_region<T> allows directly accessing an on-disk structure.
 * The region size is implicit from the type.
 */
template <typename T>
class struct_region : protected region {
   public:
    using type = typename std::decay<T>::type;
    static_assert(std::is_standard_layout_v<type>, "T must be plain-old-data type");

    /**
     * Memory map struct_region<T> at fpos in file.
     */
    struct_region(std::shared_ptr<file> f, seekoff_t fpos, len_t window = 0) : region(f, fpos, sizeof(type), window) {}

    type* get() { return static_cast<type*>(data()); }
    const type* get() const { return static_cast<const type*>(data()); }

    type* operator->() { return get(); }
    const type* operator->() const { return get(); }

    type& operator*() { return *get(); }
    const type& operator*() const { return *get(); }

    using region::flush;
    using region::getFile;
    using region::getSeek;
    using region::readAt;
    using region::sync;
    using region::writeAt;
    using region::resident;
    using region::window;
    using region::discard;

    // note: size means the sizeof(T)
    using region::size;

    /**
     * Get the file seek position just after *this.
     */
    seekoff_t getEndSeek() const { return getSeek() + sizeof(T); }

    /**
     * Seek to fpos in file and remap the region.
     * @return the pointer into the new position
     */
    type* jump(seekoff_t fpos) {
        region::jump(fpos);
        return get();
    }

    type* flushJump(seekoff_t fpos) {
        region::flushJump(fpos);
        return get();
    }
};

static_assert(std::is_move_constructible_v<struct_region<int>>);
static_assert(std::is_move_assignable_v<struct_region<int>>);
static_assert(std::is_swappable_v<struct_region<int>>);

/**
 * Typed array region.
 * @brief
 * array_region<T> allows directly accessing an on-disk array of structures
 * The element size is implicit from the type and length of the array
 * is provided by the constructor.
 * @provides resize(<elements>), operator[], begin(), end()
 */
template <typename T>
class array_region : protected region {
   protected:
    size_t num_elements = 0;

   public:
    using type = typename std::decay<T>::type;
    static_assert(std::is_standard_layout_v<type>, "T must be plain-old-data type");

    /**
     * Memory map array_region<T> at fpos in file and map array_size elements.
     */
    array_region(std::shared_ptr<file> f, seekoff_t fpos, size_t array_size) : region(f, fpos, sizeof(type) * array_size), num_elements(array_size) {}

    /**
     * Get pointer to first mapped element.
     */
    type* get() { return static_cast<type*>(data()); }
    const type* get() const { return static_cast<const type*>(data()); }

    using region::flush;
    using region::getFile;
    using region::getSeek;
    using region::readAt;
    using region::sync;
    using region::writeAt;

    /**
     * Resize the mapped array region.
     */
    void resize(size_t elements) {
        region::resize(sizeof(T) * elements);
        num_elements = elements;
    }

    /**
     * Get number of mapped *elements*
     */
    size_t size() const { return num_elements; }

    /**
     * Access the array elements
     */
    T& operator[](size_t index) {
        assert(index < num_elements);
        return get()[index];
    }
    const T& operator[](size_t index) const {
        assert(index < num_elements);
        return get()[index];
    }
    /**
     * Iterators
     */
    T* begin() { return get(); }
    T* end() { return get() + num_elements; }
    const T* begin() const { return get(); }
    const T* end() const { return get() + num_elements; }

    /**
     * Get the file seek position just after *this.
     */
    seekoff_t getEndSeek() const { return getSeek() + sizeof(T) * num_elements; }

    /**
     * Seek to fpos in file and remap the region.
     * @return the pointer into the first element in the array
     */
    type* jump(seekoff_t fpos) {
        region::jump(fpos);
        return get();
    }

    type* flushJump(seekoff_t fpos) {
        region::flushJump(fpos);
        return get();
    }
};

/**
 * Memory-mapped file I/O class.
 * @note
 * file should be created with std::make_shared<file>()
 * as mapped region(s) take shared ownership of the file.
 */
class file : public std::enable_shared_from_this<file> {
   private:
    std::mutex mut;
    int fd;
    seekoff_t fd_size;
    bool fd_rw;
    // the file and region classes are inherently coupled,
    // and we don't want to expose the internals.
    friend class region;

   public:
    enum : int {
        CREATE = 0x1,  //!< Create new file, if doesn't exist.
        RESIZE = 0x2,  //!< Resize file.
        FSTUNE = 0x4   //!< When creating new file attempt to set
                       //!< file system attributes to improve performance.
    };

    file();
    ~file();

    /**
     * Open file in read-only mode.
     * @return non-zero if error occurred.
     */
    int open(const char* file);

    /**
     * Create/Open file in read-write mode.
     * @param flags
     *  - CREATE|RESIZE creates or replaces existing file
     *    that is truncated to maxsize.
     *  - RESIZE  opens existing file and truncates it to
     *    maxsize. The file must exist already.
     *  - flags == 0 ignores the maxsize argument and opens
     *    existing file.
     * @warn default open mode discards any previous file contents!
     * @return non-zero if error occurred.
     */
    int openrw(const char* file, len_t maxsize, int flags = CREATE | RESIZE);

    /**
     * Check if file open R/W or RO
     */
    bool is_rw() const;

    /**
     * Resize the open file to newsize bytes.
     * (file must be open in R/W mode)
     * @return non-zero if error occurred.
     */
    int truncate(seekoff_t newsize);

    /**
     * Current length of the file
     * The file EOF (end-of-file) is at this position.
     */
    seekoff_t size() const;

    // Close the file.
    void close();
};

};  // namespace mapped
#endif
