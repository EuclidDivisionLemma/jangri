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
        int shell = execve("sh", NULL, NULL);
    }
    else
    {
        wait(pid);
        printf("GOOD BYE!\n");
    }

    // FILE* f = fopen("s.txt", "w");
    // fprintf(f, "HELLO\n");
    // fflush(f);
    // fclose(f);

    // f = fopen("s.txt", "r");
    // char buf[7];
    // fscanf(f, "%s", buf);
    // buf[6] = '\0';
    // printf("%s\n", buf);
    // fclose(f);

    // int pid = fork();

    // if (pid == 0)
    // {
    //     printf("SUCK DICK");
    // }
    // else
    // {
    //     printf("FUCK DICK");
    // }

    // for(;;) {}
}
