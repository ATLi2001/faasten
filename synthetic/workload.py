import json
import random
import time

def handle(req, syscall):
    args = req["args"]
    workflow = req["workflow"]
    context = req["context"]
    result = app_handle(args, context, syscall)
    if len(workflow) > 0:
        next_function = workflow.pop(0)
        syscall.invoke(next_function, json.dumps({
            "args": result,
            "workflow": workflow,
            "context": context
        }))
    return result

def app_handle(args, context, syscall):
    reps = args["reps"]
    interop_compute_ms = args["interop_compute_ms"]

    for i in range(reps):
        syscall.write_key(bytes(str(i), "utf-8"), bytes(str(random.random()), "utf-8"))
        time.sleep(interop_compute_ms / 1000)
    
    syscall.write_key(bytes("EXTERNALIZE", "utf-8"), bytes("EXTERNALIZE", "utf-8"))

    return args