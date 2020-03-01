target extended-remote :3333

# print demangled symbols
set print asm-demangle on

# set backtrace limit to not have infinite backtrace loops
set backtrace limit 32

# Enable semihosting
monitor arm semihosting enable

monitor reset

# load program
load

# start the process
continue
