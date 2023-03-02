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

# preprocess by setting up database for graderbot functions
def app_handle(args, context, syscall):
    with open("~/graderbot-functions/output/example_cos316_grader.tgz", "rb") as f:
        grading_script = f.read()
    syscall.write_key(
        bytes("cos316/example/grading_script", "utf-8"), 
        grading_script
    )

    with open("~/graderbot-functions/output/example_cos316_submission.tgz", "rb") as f:
        submission = f.read()
    syscall.write_key(
        bytes("github/cos316/example/submission.tgz", "utf-8"), 
        submission
    )

    assignments = {"example": {"grading_script": "cos316/example/grading_script", "runtime_limit": 1}}
    syscall.write_key(
        bytes("cos316/assignments", "utf-8"), 
        bytes(json.dumps(assignments), "utf-8")
    )

    config = {"test": {"TestNegate": {"points": 3}}, "subtest": {"delim": '\n'}}
    syscall.write_key(
        bytes("cos316/example/grader_config", "utf-8"), 
        bytes(json.dumps(config), "utf-8")
    )

    return args