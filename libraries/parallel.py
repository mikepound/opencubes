import multiprocessing
import math
from typing import Callable

manager = None
logging_queue = None
pool = None


def logging_task(queue: multiprocessing.Queue) -> None:
    """
    A Task function which waits for data on the log queue and prints it to the console

    Parameters:
    queue (multiprocessing.Queue): the queue to listen on
    """
    try:
        status = {}
        while True:
            (uid, done, total) = queue.get()
            status[uid] = (done, total)

            total_done = 0
            total_total = 0
            for uid, (done, total) in status.items():
                total_done += done
                total_total += total

            if (total_done == total_total):
                print(f'\rCompleted {total_done} of {total_total} {((total_done/total_total) * 100):.2f}%')
                status = {}
            else:
                print(f'\rCompleted {total_done} of {total_total} {((total_done/total_total) * 100):.2f}%', end="")
    except:
        print("multithreaded logging ended")


def init_parrelism() -> None:
    """
    helper function to initalise multiprocessing manager, queue, and logging task should only be called once
    """
    global logging_queue
    global pool
    global manager
    manager = multiprocessing.Manager()
    logging_queue = manager.Queue()
    logging_process = multiprocessing.Process(target=logging_task, args=[logging_queue])
    logging_process.daemon = True
    logging_process.start()
    pool = multiprocessing.Pool(multiprocessing.cpu_count())


def dispatch_tasks(task_function: Callable, items: list, enable_multicore: bool) -> list:
    """
    Runs a function on a list of data either sequentialy or in parallel depending on enable_multicore
    handles data chunking and recombination for you

    Parameters:
    task_function (function): the function to be mapped onto the items
    items (list): a list of arguments to your function, this will be cut into smaller lists which will be individualy sent to instances of your function
    enable_multicore (bool): whether to run the task in paralel


    Returns:
    list: a merged list of all of your functions returned lists, in order such that it is as if they where all run in one function sequentialy

    """
    # todo dont keep everything in memory, write out to files async as we go
    if (enable_multicore):
        per_core_split = math.ceil(len(items) / multiprocessing.cpu_count())
        chunk_size = min(10000, per_core_split) # split per core up to 10000
        chunk_size = max(32, chunk_size) # dont send unessicarily small chunks
        chunks = []
        for chunk_base in range(0, len(items), chunk_size):
            chunks.append((items[chunk_base: min(chunk_base + chunk_size, len(items))], logging_queue))
        items = None
        results = pool.map(task_function, chunks)
        final_result = []
        for result in reversed(results):
            if (isinstance(result, list)):
                final_result += result
            else:
                final_result += list(result)
        return final_result
    else:
        return task_function((items, None))
