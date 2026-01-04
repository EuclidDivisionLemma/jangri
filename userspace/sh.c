#include <fcntl.h>
#include <stdio.h>
#include <stdio.h>
#include <sys/unistd.h>
#include "syscalls.h"

int main(int argc, char* argv[])
{
    int pid = fork();

    if (pid == 0)
    {
        for(int i = 0; i < 1000; i++)
        {
            printf("HELLO FROM CHILD\n");
        }
    }
    else
    {
        wait(pid);
        for(int i = 0; i < 1000; i++)
        {
            printf("HELLO FROM PARENT\n");
        }
    }
}
