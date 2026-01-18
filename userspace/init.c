#include "sys/unistd.h"
#include "syscalls.h"
#include <errno.h>
#include <stdio.h>
#include <stdlib.h>

int main()
{
    int pid = fork();

    if(pid == 0)
    {
        printf("Hello from child\n");
    }
    else
    {
        wait(pid);
        printf("GOOD BYE!\n");
    }
}
