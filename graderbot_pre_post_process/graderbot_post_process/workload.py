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

# postprocess by externalizing
def app_handle(args, context, syscall):
    syscall.write_key(bytes("EXTERNALIZE", "utf-8"), bytes("EXTERNALIZE", "utf-8"))

    resp = syscall.read_key(bytes(args["report"], "utf-8"))

    return resp
