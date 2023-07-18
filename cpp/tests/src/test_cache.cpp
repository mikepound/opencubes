#include <gtest/gtest.h>

#include "cache.hpp"

TEST(CacheTests, TestCacheLoadDoesNotThrow) { EXPECT_NO_THROW(Cache::load("./test_data.bin")); }

TEST(CacheTests, TestCacheSaveDoesNotThrow) {
    auto data = Cache::load("./test_data.bin");
    EXPECT_NO_THROW(Cache::save("./temp.bin", data, 255));
}