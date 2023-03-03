import json

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
    seed = args["f2_seed"]
    if seed < 1: 
        syscall.write_key(bytes("f2", "utf-8"), bytes("v2", "utf-8"))
        args["f2"] = "v2"
    else:
        prev_val = syscall.read_key(bytes("f2", "utf-8")).decode("utf-8")
        new_key = "f2_%s_%s" % (prev_val, context["time"])
        syscall.write_key(bytes(new_key, "utf-8"), bytes("v2", "utf-8"))
        args[new_key] = "v2"

    args["f2_seed"] = seed + 1

    return args