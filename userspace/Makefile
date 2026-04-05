CC ?= gcc
LD ?= ld
READELF ?= readelf

# Produce ET_DYN binaries for LeonOS native relocate runner.
CFLAGS := -ffreestanding -fno-stack-protector -fcf-protection=none -fPIE -Os -nostdlib -nostartfiles -Wall -Wextra
ASFLAGS := -ffreestanding -fPIE -nostdlib -nostartfiles
LDFLAGS := -shared -e _start -z max-page-size=0x1000

BINS := hello_world c_hello_name exit7 wait4_echild wait4_reap
OBJS := hello_world.o c_hello_name.o exit7.o wait4_echild.o wait4_reap.o

all: $(BINS) check-type

hello_world: hello_world.o
	$(LD) $(LDFLAGS) -o $@ $<

c_hello_name: c_hello_name.o
	$(LD) $(LDFLAGS) -o $@ $<

exit7: exit7.o
	$(LD) $(LDFLAGS) -o $@ $<

wait4_echild: wait4_echild.o
	$(LD) $(LDFLAGS) -o $@ $<

wait4_reap: wait4_reap.o
	$(LD) $(LDFLAGS) -o $@ $<

%.o: %.c
	$(CC) $(CFLAGS) -c -o $@ $<

hello_world.o: hello_world.S
	$(CC) $(ASFLAGS) -c -o $@ $<

check-type:
	@$(READELF) -h exit7 | grep "Type:" || true
	@$(READELF) -h c_hello_name | grep "Type:" || true
	@$(READELF) -h wait4_echild | grep "Type:" || true
	@$(READELF) -h wait4_reap | grep "Type:" || true

clean:
	rm -f $(BINS) $(OBJS)
