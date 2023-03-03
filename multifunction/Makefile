FUNCTIONS=f1 f2 f3
OUTPUTS=$(patsubst %, %/output_%.ext2, $(FUNCTIONS))

all: $(OUTPUTS)

%/output_%.ext2: %/*
	make -C %/*
