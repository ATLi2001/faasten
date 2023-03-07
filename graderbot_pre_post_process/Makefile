all: graderbot_pre_process/output.ext2 graderbot_post_process/output.ext2

graderbot_pre_process/output.ext2: graderbot_pre_process/workload.py
	make -C graderbot_pre_process

graderbot_post_process/output.ext2: graderbot_post_process/workload.py
	make -C graderbot_post_process
