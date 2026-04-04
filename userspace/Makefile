CC ?= gcc

AS_FLAGS := -nostdlib -static -s
C_FLAGS := -nostdlib -static -s -Os -fno-stack-protector -fno-pie -no-pie

all: hello_world c_hello_name

hello_world:
	$(CC) $(AS_FLAGS) -o hello_world hello_world.S

c_hello_name:
	$(CC) $(C_FLAGS) -Wl,-e,_start -o c_hello_name c_hello_name.c

clean:
	rm -f hello_world c_hello_name
