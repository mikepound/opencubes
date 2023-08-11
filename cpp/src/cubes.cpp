#include "cubes.hpp"

#include <algorithm>
#include <chrono>
#include <cstdint>
#include <filesystem>
#include <iostream>
#include <mutex>
#include <thread>
#include <deque>
#include <condition_variable>

#include "cache.hpp"
#include "cube.hpp"
#include "hashes.hpp"
#include "newCache.hpp"
#include "results.hpp"
#include "rotations.hpp"

const int PERF_STEP = 500;

struct Workset {
    std::mutex mu;

    CacheReader cr;
    CubeIterator _begin_total;
    CubeIterator _begin;
    CubeIterator _end;
    Hashy &hashes;
    XYZ targetShape, shape, expandDim;
    bool notSameShape;
    Workset(Hashy &hashes, XYZ targetShape, XYZ shape, XYZ expandDim, bool notSameShape)
        : hashes(hashes)
        , targetShape(targetShape)
        , shape(shape)
        , expandDim(expandDim)
        , notSameShape(notSameShape) {}

    void setRange(ShapeRange &data) {
        _begin_total = data.begin();
        _begin = data.begin();
        _end = data.end();
    }

    struct Subset {
        CubeIterator _begin, _end;
        bool valid;
        float percent;
        auto begin() { return _begin; }
        auto end() { return _end; }
    };

    Subset getPart() {
        std::lock_guard<std::mutex> g(mu);
        auto a = _begin;
        _begin += 500;
        if (_begin > _end) _begin = _end;
        return {a, _begin, a < _end, 100 * float(std::distance(_begin_total.data(), a.data())) / std::distance(_begin_total.data(), _end.data())};
    }

    void expand(const Cube &c) {
        std::vector<XYZ> candidates, tmp;
        candidates.reserve(c.size() * 6);

        if (notSameShape) {
            for (const auto &p : c) {
                if (expandDim.x() == 1) {
                    if (p.x() == shape.x()) candidates.emplace_back(XYZ(p.x() + 1, p.y(), p.z()));
                    if (p.x() == 0) candidates.emplace_back(XYZ(p.x() - 1, p.y(), p.z()));
                }
                if (expandDim.y() == 1) {
                    if (p.y() == shape.y()) candidates.emplace_back(XYZ(p.x(), p.y() + 1, p.z()));
                    if (p.y() == 0) candidates.emplace_back(XYZ(p.x(), p.y() - 1, p.z()));
                }
                if (expandDim.z() == 1) {
                    if (p.z() == shape.z()) candidates.emplace_back(XYZ(p.x(), p.y(), p.z() + 1));
                    if (p.z() == 0) candidates.emplace_back(XYZ(p.x(), p.y(), p.z() - 1));
                }
            }
        } else {
            for (const auto &p : c) {
                if (p.x() < shape.x()) candidates.emplace_back(XYZ(p.x() + 1, p.y(), p.z()));
                if (p.x() > 0) candidates.emplace_back(XYZ(p.x() - 1, p.y(), p.z()));
                if (p.y() < shape.y()) candidates.emplace_back(XYZ(p.x(), p.y() + 1, p.z()));
                if (p.y() > 0) candidates.emplace_back(XYZ(p.x(), p.y() - 1, p.z()));
                if (p.z() < shape.z()) candidates.emplace_back(XYZ(p.x(), p.y(), p.z() + 1));
                if (p.z() > 0) candidates.emplace_back(XYZ(p.x(), p.y(), p.z() - 1));
            }
        }
        std::sort(candidates.begin(), candidates.end());
        auto end = std::unique(candidates.begin(), candidates.end());
        // Copy XYZ not in Cube into tmp
        tmp.reserve(std::distance(candidates.begin(), end));
        std::set_difference(candidates.begin(), end, c.begin(), c.end(), std::back_inserter(tmp));
        candidates = std::move(tmp);

        DEBUG1_PRINTF("candidates: %lu\n\r", candidates.size());

        Cube newCube(c.size() + 1);
        Cube lowestHashCube(newCube.size());
        Cube rotatedCube(newCube.size());

        for (const auto &p : candidates) {
            DEBUG2_PRINTF("(%2d %2d %2d)\n\r", p.x(), p.y(), p.z());
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
            // check rotations
            XYZ lowestShape;
            bool none_set = true;
            for (int i = 0; i < 24; ++i) {
                auto [res, ok] = Rotations::rotate(i, shape, newCube, rotatedCube);
                if (!ok) continue;  // rotation generated violating shape

                std::sort(rotatedCube.begin(), rotatedCube.end());

                if (none_set || lowestHashCube < rotatedCube) {
                    none_set = false;
                    swap(lowestHashCube, rotatedCube);
                    lowestShape = res;
                }
            }
            hashes.insert(lowestHashCube, lowestShape);
        }
    }
};

struct Worker {
    std::shared_ptr<Workset> ws;
    int id;
    int state = 3; // 1 == completed/waiting for job, 2 == processing, 3 == job assigned.
    std::mutex mtx;
    std::condition_variable cond;
    std::condition_variable cond2;
    std::thread thr;

    Worker(int id_) : id(id_), thr(&Worker::run, this) {}
    ~Worker() {
        std::unique_lock lock(mtx);
        state = 0;
        cond.notify_one();
        lock.unlock();
        thr.join();
    }

    void launch(std::shared_ptr<Workset> ws_) {
        std::unique_lock lock(mtx);
        while(state > 1) {
            cond2.wait(lock);
        }
        ws = ws_;
        state = 3;
        cond.notify_one();
    }

    void sync() {
        std::unique_lock lock(mtx);
        while(state > 1) {
            cond2.wait(lock);
        }
        ws.reset();
    }

    void run() {
        std::unique_lock lock(mtx);
        std::printf("thread nro. %d started.\n", id);
        while(state) {
            state = 1;
            cond2.notify_one();
            while(state == 1)
                cond.wait(lock);
            if(!state)
                return;
            state = 2;
            // std::printf("start %d\n", id);
            auto subset = ws->getPart();
            while (subset.valid) {
                if (id == 0) {
                    std::printf("  %5.2f%%\r", subset.percent);
                    std::flush(std::cout);
                }
                // std::cout << id << " next subset " << &*subset.begin() << " to " << &*subset.end() << "\n";
                for (auto &c : subset) {
                    // std::printf("%p\n", (void *)&c);
                    // c.print();
                    ws->expand(c);
                }
                subset = ws->getPart();
            }
            // std::printf("finished %d\n", id);
        }
    }
};

FlatCache gen(int n, int threads, bool use_cache, bool write_cache, bool split_cache, bool use_split_cache, std::string base_path) {
    if (!std::filesystem::is_directory(base_path)) {
        std::filesystem::create_directory(base_path);
    }
    Hashy hashes;
    if (n < 1)
        return {};
    else if (n == 1) {
        hashes.init(n);
        hashes.insert(Cube{{XYZ(0, 0, 0)}}, XYZ(0, 0, 0));
        std::printf("%ld elements for %d\n\r", hashes.size(), n);
        if (write_cache) {
            Cache::save(base_path + "cubes_" + std::to_string(n) + ".bin", hashes, n);
        }
        return FlatCache(hashes, n);
    }

    CacheReader cr;
    if (use_cache && !use_split_cache) {
        std::string cachefile = base_path + "cubes_" + std::to_string(n - 1) + ".bin";
        cr.loadFile(cachefile);
        cr.printHeader();
    }
    FlatCache fc;
    ICache *base = &cr;
    if (!cr && !use_split_cache) {
        fc = gen(n - 1, threads, use_cache, write_cache, false);
        base = &fc;
    }
    std::printf("N = %d || generating new cubes from %lu base cubes.\n\r", n, base->size());
    hashes.init(n);

    // Start worker threads.
    std::deque<Worker> workers;
    for (int i = 0; i < threads; ++i) {
        workers.emplace_back(i);
    }


    uint64_t totalSum = 0;
    auto start = std::chrono::steady_clock::now();
    uint32_t totalOutputShapes = hashes.byshape.size();
    uint32_t outShapeCount = 0;

    auto prevShapes = Hashy::generateShapes(n - 1);
    for (auto &tup : hashes.byshape) {
        outShapeCount++;
        XYZ targetShape = tup.first;
        std::printf("process output shape %3d/%d [%2d %2d %2d]\n\r", outShapeCount, totalOutputShapes, targetShape.x(), targetShape.y(), targetShape.z());
        for (uint32_t sid = 0; sid < prevShapes.size(); ++sid) {
            auto &shape = prevShapes[sid];
            int diffx = targetShape.x() - shape.x();
            int diffy = targetShape.y() - shape.y();
            int diffz = targetShape.z() - shape.z();
            int abssum = abs(diffx) + abs(diffy) + abs(diffz);
            if (abssum > 1 || diffx < 0 || diffy < 0 || diffz < 0) {
                continue;
            }
            // handle symmetry cases
            if (diffz == 1) {
                if (shape.z() == shape.y()) diffy = 1;
            }
            if (diffy == 1)
                if (shape.y() == shape.x()) diffx = 1;

            auto ws = std::make_shared<Workset>(hashes, targetShape, shape, XYZ(diffx, diffy, diffz), abssum);

            if (use_split_cache) {
                // load cache file only for this shape
                std::string cachefile = base_path + "cubes_" + std::to_string(n - 1) + "_" + std::to_string(prevShapes[sid].x()) + "-" +
                                        std::to_string(prevShapes[sid].y()) + "-" + std::to_string(prevShapes[sid].z()) + ".bin";
                ws->cr.loadFile(cachefile);
                base = &ws->cr;
                // cr.printHeader();
            }
            auto s = base->getCubesByShape(sid);
            if (shape != s.shape()) {
                std::printf("ERROR caches shape does not match expected shape!\n");
                exit(-1);
            }

            ws->setRange(s);

            // Wait for jobs to complete.
            for (auto& thr : workers) {
                thr.sync();
            }
            std::printf("  shape %d %d %d\n\r", shape.x(), shape.y(), shape.z());
            // launch the new jobs.
            // Because the workset is held by shared_ptr
            // main thread can do above preparation work in parallel
            // while the jobs are running.
            for (auto& thr : workers) {
                thr.launch(ws);
            }
        }
        // Wait for jobs to complete.
        for (auto& thr : workers) {
            thr.sync();
        }
        std::printf("  num: %lu\n\r", hashes.byshape[targetShape].size());
        totalSum += hashes.byshape[targetShape].size();
        if (write_cache && split_cache) {
            Cache::save(base_path + "cubes_" + std::to_string(n) + "_" + std::to_string(targetShape.x()) + "-" + std::to_string(targetShape.y()) + "-" +
                            std::to_string(targetShape.z()) + ".bin",
                        hashes, n);
        }
        if (split_cache) {
            for (auto &subset : hashes.byshape[targetShape].byhash) {
                subset.set.clear();
                subset.set.reserve(1);
            }
        }
    }

    // Stop the workers.
    workers.clear();

    if (write_cache && !split_cache) {
        Cache::save(base_path + "cubes_" + std::to_string(n) + ".bin", hashes, n);
    }
    auto end = std::chrono::steady_clock::now();
    auto dt_ms = std::chrono::duration_cast<std::chrono::milliseconds>(end - start).count();
    std::printf("took %.2f s\033[0K\n\r", dt_ms / 1000.f);
    std::printf("num total cubes: %lu\n\r", totalSum);
    checkResult(n, totalSum);
    return FlatCache(hashes, n);
}
