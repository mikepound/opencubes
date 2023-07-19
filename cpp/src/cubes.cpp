#include "cubes.hpp"

#include <algorithm>
#include <chrono>
#include <cstdint>
#include <iostream>
#include <thread>

#include "cache.hpp"
#include "cube.hpp"
#include "hashes.hpp"
#include "results.hpp"
#include "rotations.hpp"

const int PERF_STEP = 500;

void expand(const Cube &c, Hashy &hashes) {
    // Get expanded Cube XYZ
    std::vector<XYZ> candidates, tmp;
    candidates.reserve(c.size() * 6);
    for (const auto &p : c) {
        candidates.emplace_back(XYZ(p.x() + 1, p.y(), p.z()));
        candidates.emplace_back(XYZ(p.x() - 1, p.y(), p.z()));
        candidates.emplace_back(XYZ(p.x(), p.y() + 1, p.z()));
        candidates.emplace_back(XYZ(p.x(), p.y() - 1, p.z()));
        candidates.emplace_back(XYZ(p.x(), p.y(), p.z() + 1));
        candidates.emplace_back(XYZ(p.x(), p.y(), p.z() - 1));
    }
    std::sort(candidates.begin(), candidates.end());
    auto end = std::unique(candidates.begin(), candidates.end());
    // Copy XYZ not in Cube into tmp
    tmp.reserve(std::distance(candidates.begin(), end));
    std::set_difference(candidates.begin(), end, c.begin(), c.end(), std::back_inserter(tmp));
    candidates = std::move(tmp);

    DEBUG_PRINTF("candidates: %lu\n\r", candidates.size());

    Cube newCube(c.size() + 1);
    Cube lowestHashCube(newCube.size());
    Cube rotatedCube(newCube.size());

    for (const auto &p : candidates) {
        DEBUG_PRINTF("(%2d %2d %2d)\n\r", p.x(), p.y(), p.z());
        int ax = (p.x() < 0) ? 1 : 0;
        int ay = (p.y() < 0) ? 1 : 0;
        int az = (p.z() < 0) ? 1 : 0;
        auto put = newCube.begin();
        *put++ = XYZ(p.x() + ax, p.y() + ay, p.z() + az);
        XYZ shape(p.x() + ax, p.y() + ay, p.z() + az);
        for (const auto &np : c) {
            auto nx = np.x() + ax;
            auto ny = np.y() + ay;
            auto nz = np.z() + az;
            if (nx > shape[0]) shape[0] = nx;
            if (ny > shape[1]) shape[1] = ny;
            if (nz > shape[2]) shape[2] = nz;
            *put++ = XYZ(nx, ny, nz);
        }
        DEBUG_PRINTF("shape %2d %2d %2d\n\r", shape[0], shape[1], shape[2]);

        // check rotations
        XYZ lowestShape;
        bool none_set = true;
        for (int i = 0; i < 24; ++i) {
            auto [res, ok] = Rotations::rotate(i, shape, newCube, rotatedCube);
            if (!ok) continue;  // rotation generated violating shape

            std::sort(rotatedCube.begin(), rotatedCube.end());

            if (none_set || lowestHashCube < rotatedCube) {
                none_set = false;
                // std::printf("shape %2d %2d %2d\n\r", res.first.x(), res.first.y(), res.first.z());
                swap(lowestHashCube, rotatedCube);
                lowestShape = res;
            }
        }
        hashes.insert(lowestHashCube, lowestShape);
        DEBUG_PRINTF("inserted! (num %2lu)\n\n\r", hashes.size());
    }
    DEBUG_PRINTF("new hashes: %lu\n\r", hashes.size());
}

void expandPart(std::vector<Cube> &base, Hashy &hashes, size_t start, size_t end) {
    auto t_start = std::chrono::steady_clock::now();
    auto t_last = t_start;
    auto total = end - start;
    for (auto i = start; i < end; ++i) {
        expand(base[i], hashes);
        auto count = i - start;
        if (start == 0 && (count % PERF_STEP == (PERF_STEP - 1))) {
            auto t_end = std::chrono::steady_clock::now();
            auto total_us = std::chrono::duration_cast<std::chrono::microseconds>(t_end - t_start).count();
            auto dt_us = std::chrono::duration_cast<std::chrono::microseconds>(t_end - t_last).count();
            t_last = t_end;
            auto perc = 100 * count / total;
            auto avg = 1000000.f * count / total_us;
            auto its = 1000000.f * PERF_STEP / dt_us;
            auto remaining = (end - i) / avg;
            std::printf(" %3ld%%, %5.0f avg baseCubes/s, %5.0f baseCubes/s, remaining: %.0fs\033[0K\r", perc, avg, its, remaining);
            std::flush(std::cout);
        }
    }
    auto t_end = std::chrono::steady_clock::now();
    auto dt_ms = std::chrono::duration_cast<std::chrono::milliseconds>(t_end - t_start).count();
    std::printf("  done took %.2f s [%7lu, %7lu]\033[0K\n\r", dt_ms / 1000.f, start, end);
}

Hashy gen(int n, int threads, bool use_cache, bool write_cache) {
    Hashy hashes;
    if (n < 1)
        return {};
    else if (n == 1) {
        hashes.init(n);
        hashes.insert(Cube{{XYZ(0, 0, 0)}}, XYZ(0, 0, 0));
        std::printf("%ld elements for %d\n\r", hashes.size(), n);
        return hashes;
    } else if (n == 2) {
        hashes.init(2);
        hashes.insert(Cube{XYZ(0, 0, 0), XYZ(0, 0, 1)}, XYZ(0, 0, 1));
        std::printf("%ld elements for %d\n\r", hashes.size(), n);
        return hashes;
    }

    if (use_cache) {
        hashes = Cache::load("cubes_" + std::to_string(n) + ".bin");

        if (hashes.size() != 0) return hashes;
    }

    auto base = gen(n - 1, threads, use_cache, write_cache);
    std::printf("N = %d || generating new cubes from %lu base cubes.\n\r", n, base.size());
    hashes.init(n);
    int count = 0;
    if (threads == 1 || base.size() < 100) {
        auto start = std::chrono::steady_clock::now();
        auto last = start;
        int total = base.size();

        for (const auto &s : base.byshape) {
            // std::printf("shapes %d %d %d\n\r", s.first.x(), s.first.y(), s.first.z());
            for (const auto &subset : s.second.byhash)
                for (const auto &b : subset.set) {
                    expand(b, hashes);
                    count++;
                    if (count % PERF_STEP == (PERF_STEP - 1)) {
                        auto end = std::chrono::steady_clock::now();
                        auto total_us = std::chrono::duration_cast<std::chrono::microseconds>(end - start).count();
                        auto dt_us = std::chrono::duration_cast<std::chrono::microseconds>(end - last).count();
                        last = end;
                        auto perc = 100 * count / total;
                        auto avg = 1000000.f * count / total_us;
                        auto its = 1000000.f * PERF_STEP / dt_us;
                        auto remaining = (total - count) / avg;
                        std::printf(" %3d%%, %5.0f avg baseCubes/s, %5.0f baseCubes/s, remaining: %.0fs\033[0K\r", perc, avg, its, remaining);
                        std::flush(std::cout);
                    }
                }
        }
        auto end = std::chrono::steady_clock::now();
        auto dt_ms = std::chrono::duration_cast<std::chrono::milliseconds>(end - start).count();
        std::printf("  took %.2f s\033[0K\n\r", dt_ms / 1000.f);
    } else {
        std::vector<Cube> baseCubes;
        std::printf("converting to vector\n\r");
        for (auto &s : base.byshape)
            for (auto &subset : s.second.byhash) {
                baseCubes.insert(baseCubes.end(), subset.set.begin(), subset.set.end());
                subset.set.clear();
                subset.set.reserve(1);
            }
        std::printf("starting %d threads\n\r", threads);
        std::vector<std::thread> ts;
        ts.reserve(threads);
        for (int i = 0; i < threads; ++i) {
            auto start = baseCubes.size() * i / threads;
            auto end = baseCubes.size() * (i + 1) / threads;

            ts.emplace_back(expandPart, std::ref(baseCubes), std::ref(hashes), start, end);
        }
        for (int i = 0; i < threads; ++i) {
            ts[i].join();
        }
    }
    std::printf("  num cubes: %lu\n\r", hashes.size());
    if (write_cache) Cache::save("cubes_" + std::to_string(n) + ".bin", hashes, n);
    if (sizeof(results) / sizeof(results[0]) > ((uint64_t)(n - 1)) && n > 1) {
        if (results[n - 1] != hashes.size()) {
            std::printf("ERROR: result does not equal resultstable (%lu)!\n\r", results[n - 1]);
            std::exit(-1);
        }
    }
    return hashes;
}
