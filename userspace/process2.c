#include "sys/unistd.h"
#include <stdio.h>
#include <stdlib.h>

int factorial(int x)
{
    if (x == 0 || x == 1)
    {
        return 1;
    }

    return x * factorial(x - 1);
}

int main()
{
    for (;;)
    {
        printf("\nProcess 2\n");
        int* x = malloc(sizeof(int));
        printf("Enter a number: ");
        scanf("%d", x);
        int fact = factorial(*x);
        printf("The factorial of %d is %d\n", *x, fact);
        free(x);
    }
}
