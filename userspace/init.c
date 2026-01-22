#include <stdio.h>
#include <stdlib.h>

int main()
{
    int* x = malloc(sizeof(int));
    printf("Enter your age: ");
    scanf("%d", x);

    if (*x < 18)
    {
        printf("You are not eligible to vote\n");
    }
    else
    {
        printf("You are indeed eligible\n");
    }
}
