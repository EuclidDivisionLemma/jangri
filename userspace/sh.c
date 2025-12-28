#include <fcntl.h>
#include <stdio.h>
#include <sys/unistd.h>

int main(int argc, char* argv[])
{
    int fd = open("s.txt", O_CREAT | O_RDWR);

    if (fd < 0)
    {
        write(2, "ERROR OPENING FILE", 19);
    }

    write(fd, "I've been wishing on a falling star for too long. I've been running, i do not know what from", 92);
    lseek(fd, -10, 2);

    lseek(fd, 100, SEEK_CUR);
    if(lseek(fd, -100, SEEK_CUR) < 0)
    {
        write(1, "FAILED", 6);
    }

    char buf[193];
    read(fd, buf, 193);

    write(1, buf, 193);

    for (;;)
    {
    }
}
