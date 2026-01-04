#include "syscalls.h"
#include <errno.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/types.h>
#include <sys/unistd.h>

int open(const char* path, int flag)
{
    register ssize_t a7 asm("a7") = 100;
    register ssize_t a0 asm("a0") = (size_t)path;
    register ssize_t a1 asm("a1") = flag;

    asm volatile("ecall" : "+r"(a0) : "r"(a7), "r"(a1) : "memory");

    if (a0 >= 0)
    {
        return a0;
    }

    errno = -a0;
    return -1;
}

int close(int fd)
{

    register ssize_t a0 asm("a0") = fd;
    register ssize_t a7 asm("a7") = 400;

    asm volatile("ecall" : "+r"(a0) : "r"(a7) : "memory");

    if (a0 >= 0)
        return a0;

    errno = -a0;
    return -1;
}

ssize_t read(int fd, void* buf, size_t num_bytes)
{
    register ssize_t a0 asm("a0") = fd;
    register ssize_t a7 asm("a7") = 200;
    register ssize_t a1 asm("a1") = (ssize_t)buf;
    register ssize_t a2 asm("a2") = num_bytes;

    asm volatile("ecall" : "+r"(a0) : "r"(a7), "r"(a1), "r"(a2) : "memory");

    if (a0 >= 0)
        return a0;

    errno = -a0;
    return -1;
}

ssize_t write(int fd, const void* buf, size_t num_bytes)
{
    register size_t a7 asm("a7") = 300;
    register ssize_t a0 asm("a0") = fd;
    register const void* a1 asm("a1") = buf;
    register size_t a2 asm("a2") = num_bytes;

    asm volatile("ecall" : "+r"(a0) : "r"(a7), "r"(a1), "r"(a2) : "memory");

    if (a0 >= 0)
        return a0;

    errno = -a0;
    return -1;
}

off_t lseek(int fd, off_t offset, int whence)
{
    register ssize_t a7 asm("a7") = 500;
    register ssize_t a0 asm("a0") = fd;
    register ssize_t a1 asm("a1") = offset;
    register ssize_t a2 asm("a2") = whence;

    asm volatile("ecall" : "+r"(a0) : "r"(a7), "r"(a1), "r"(a2) : "memory");

    if (a0 >= 0)
        return a0;

    errno = -a0;
    return offset - 1;
}

void* sbrk(ptrdiff_t increment)
{
    asm volatile("li a7, 700\n"
                 "mv a0, %0\n"
                 "ecall"
                 :
                 : "r"(increment));

    ssize_t a0;
    asm volatile("mv %0, a0" : "=r"(a0));

    if (a0 >= 0)
        return (void*)a0;

    errno = -a0;
    return (void*)-1;
}

int pipe(int fd[2])
{
    register ssize_t a7 asm("a7") = 600;
    register ssize_t a0 asm("a0") = (ssize_t)fd;

    asm volatile("ecall" : "+r"(a0) : "r"(a7) : "memory");

    if (a0 >= 0)
        return a0;

    errno = -a0;
    return -1;
}

void exit(int r)
{
    register ssize_t a7 asm("a7") = 800;
    register ssize_t a0 asm("a0") = r;

    asm volatile("ecall" : : "r"(a7), "r"(a0));
}

pid_t fork()
{
    register ssize_t a7 asm("a7") = 900;
    register ssize_t a0 asm("a0") = 0;

    asm volatile("ecall" : "+r"(a0) : "r"(a7) : "memory");

    if (a0 >= 0)
    {
        return a0;
    }
    else
    {
        errno = -a0;
        return -1;
    }
}

pid_t wait(pid_t pid)
{
    register ssize_t a7 asm("a7") = 1000;
    register ssize_t a0 asm("a0") = pid;

    asm volatile("ecall" : "+r"(a0) : "r"(a7) : "memory");

    if (a0 >= 0)
    {
        return a0;
    }

    errno = -a0;
    return -1;
}
