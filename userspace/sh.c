#include <fcntl.h>
#include <stdio.h>
#include <string.h>
#include <sys/unistd.h>

int main(int argc, char* argv[])
{
    int x;
    scanf("%d", &x);
    printf("%d\n", x);
    write(1, "JELLO\n", 6);

    for (;;)
    {
    }
}
