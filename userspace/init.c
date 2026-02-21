#include "sys/unistd.h"
#include <stdio.h>
#include <stdlib.h>

int main()
{
    int fds[2];
    pipe(fds);

    write(fds[1], "Together Together Together Everyone", 36);
    char buf[36];

    read(fds[0], buf, 36);

    printf("%s\n", buf);
}
