// Copyright 2026 Aadarsh

// Permission is hereby granted, free of charge, to any person obtaining a copy of this software
// and associated documentation files (the “Software”), to deal in the Software without restriction,
// including without limitation the rights to use, copy, modify, merge, publish, distribute,
// sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:

// The above copyright notice and this permission notice shall be included in all copies or
// substantial portions of the Software.

// THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED,
// INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR
// PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR
// ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE,
// ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.


// cat
// Adapted from xv6's cat.c to Jangri

#include "sys/_default_fcntl.h"
#include "sys/unistd.h"
#include <stdio.h>
#include <stdlib.h>

char buf[512];

void cat(int fd)
{
    int n;

    while ((n = read(fd, buf, sizeof(buf))) > 0)
    {
        if (write(1, buf, n) != n)
        {
            printf("cat: write error\n");
            exit(1);
        }
    }
    if (n < 0)
    {
        printf("cat: read error\n");
        exit(1);
    }
}

int main(int argc, char* argv[])
{
    int fd, i;

    if (argc <= 1)
    {
        cat(0);
        exit(0);
    }

    for (i = 1; i < argc; i++)
    {
        if ((fd = open(argv[i], O_RDONLY)) < 0)
        {
            printf("cat: cannot open %s\n", argv[i]);
            exit(1);
        }
        cat(fd);
        close(fd);
    }
    exit(0);
}
