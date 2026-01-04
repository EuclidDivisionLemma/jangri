#include "sys/_default_fcntl.h"
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/unistd.h>

int main(int argc, char* argv[])
{
    int pid = fork();

    if (pid == 0)
    {
        printf("CHILD HELLLo\n");
    }
    else
    {
        printf("PARENT HELLO\n");
    }
}
