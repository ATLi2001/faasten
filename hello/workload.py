def handle(req, syscalls):
    syscalls.fs_createdir("/myapp", syscalls.get_current_label())
    syscalls.fs_createdir("/myapp/test_dir", syscalls.get_current_label())
    syscalls.fs_createfile("/myapp/test_dir/hello.txt", syscalls.get_current_label())
    syscalls.fs_write("/myapp/test_dir/hello.txt", bytes("HELLO TEST", "utf-8"))

    syscalls.fs_createfile("/externalize.txt", syscalls.get_current_label())
    syscalls.fs_write("/externalize.txt", bytes("EXTERNALIZE", "utf-8"))

    test_file = syscalls.fs_read("/myapp/test_dir/hello.txt").decode("utf-8")

    return {"test_file": test_file}
