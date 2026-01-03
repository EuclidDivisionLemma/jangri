#include "sys/_default_fcntl.h"
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/unistd.h>

int main(int argc, char* argv[])
{
    int fd = open("s.txt", O_CREAT | O_WRONLY);

    printf("%d\n", fd);


    write(fd, "HELLooooooO\n", 13);

    char* buf = malloc(6);

    if (lseek(fd, 0, SEEK_SET) == -1)
    {
        write(2, "ERROR\n", 5);
        return -1;
    }


    read(fd, buf, 6);

    write(1, buf, 6);

    for (;;)
    {
    }
}
