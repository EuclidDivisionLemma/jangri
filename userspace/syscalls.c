#include <errno.h>
#include <sys/types.h>
#include <sys/unistd.h>

int open(const char* path, int flag)
{
    asm volatile("li a7, 100");
    asm volatile("mv a0, %0" ::"r"(path));
    asm volatile("mv a1, %0" ::"r"(flag));
    asm volatile("ecall");

    size_t a0;

    asm volatile("mv %0, a0" : "=r"(a0));

    if (a0 >= 0)
    {
        return a0;
    }

    errno = -a0;
    return -1;
}

int close(int fd)
{
    asm volatile("li a7, 400\n"
                 "mv a0, %0\n"
                 "ecall"
                 :
                 : "r"(fd));

    size_t a0;
    asm volatile("mv %0, a0" : "=r"(a0));

    if (a0 >= 0)
        return a0;

    errno = -a0;
    return -1;
}

ssize_t read(int fd, void* buf, size_t num_bytes)
{
    asm volatile("li a7, 200\n"
                 "mv a0, %0\n"
                 "mv a1, %1\n"
                 "mv a2, %2\n"
                 "ecall"
                 :
                 : "r"(fd), "r"(buf), "r"(num_bytes));
    size_t a0;
    asm volatile("mv %0, a0" : "=r"(a0));

    if (a0 >= 0)
        return a0;

    errno = -a0;
    return -1;
}

ssize_t write(int fd, const void* buf, size_t num_bytes)
{
    asm volatile("li a7, 300\n"
                 "mv a0, %0\n"
                 "mv a1, %1\n"
                 "mv a2, %2\n"
                 "ecall"
                 :
                 : "r"(fd), "r"(buf), "r"(num_bytes));

    size_t a0;
    asm volatile("mv %0, a0" : "=r"(a0));

    if (a0 >= 0)
        return a0;

    errno = -a0;
    return -1;
}

off_t lseek(int fd, off_t offset, int whence)
{
    asm volatile("li a7, 500\n"
                 "mv a0, %0\n"
                 "mv a1, %1\n"
                 "mv a2, %2\n"
                 "ecall"
                 :
                 : "r"(fd), "r"(offset), "r"(whence));

    size_t a0;
    asm volatile("mv %0, a0" : "=r"(a0));

    if (a0 >= 0)
        return a0;

    errno = -a0;
    return offset - 1;
}

void* sbrk(ptrdiff_t increment)
{
    return NULL;
}

int pipe(int fd[2])
{
    asm volatile("li a7, 600\n"
                 "mv a0, %0\n"
                 "ecall"
                 :
                 : "r"(fd));

    size_t a0;
    asm volatile("mv %0, a0" : "=r"(a0));

    if (a0 >= 0)
        return a0;

    errno = -a0;
    return -1;
}
