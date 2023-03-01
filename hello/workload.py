def handle(req, syscalls):

    for i in range(10):
        syscalls.write_key(bytes("key%d" % i, "utf-8"), bytes("val%d" % i, "utf-8"))
    
    syscalls.write_key(bytes("EXTERNALIZE", "utf-8"), bytes("EXTERNALIZE", "utf-8"))

    for i in range(10):
        resp = syscalls.read_key(bytes("key%d" % i, "utf-8")).decode("utf-8")
        if resp != ("val%d" % i):
            return {"error": "Got response %s from %s" % (resp, "key%d" % i)}
        
    return {"success": "10 keys write/read"}

    # syscalls.fs_createdir("/myapp", syscalls.get_current_label())
    # syscalls.fs_createdir("/myapp/test_dir", syscalls.get_current_label())

    # for i in range(10):
    #     syscalls.fs_createfile("myapp/test_dir/file%d.txt" % i, syscalls.get_current_label())
    #     for j in range(10):
    #         syscalls.fs_write("/myapp/test_dir/file%d.txt" % i, bytes("j=%d" % j, "utf-8"))

    # for i in range(10):
    #     resp_i = syscalls.fs_read("/myapp/test_dir/file%d.txt" % i).decode("utf-8")
    #     if resp_i != "j=9":
    #         return {"error": "Got response %s from %s" % (resp_i, "file%d.txt" % i)}
    
    # syscalls.fs_createfile("/externalize.txt", syscalls.get_current_label())
    # syscalls.fs_write("/externalize.txt", bytes("EXTERNALIZE", "utf-8"))

    # return {"success": "SUCCESS"}

    # syscalls.fs_createfile("/myapp/test_dir/hello.txt", syscalls.get_current_label())
    # syscalls.fs_write("/myapp/test_dir/hello.txt", bytes("HELLO TEST", "utf-8"))

    # syscalls.fs_createfile("/externalize.txt", syscalls.get_current_label())
    # syscalls.fs_write("/externalize.txt", bytes("EXTERNALIZE", "utf-8"))

    # test_file = syscalls.fs_read("/myapp/test_dir/hello.txt").decode("utf-8")

    # return {"test_file": test_file}
