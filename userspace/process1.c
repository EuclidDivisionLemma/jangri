#include <stdio.h>
#include <stdlib.h>

int main()
{
    for (;;)
    {
        printf("Process 1:\n");
        int* x = (int*)malloc(sizeof(int));
        printf("Enter your age: ");
        scanf("%d", x);

        if (*x < 18)
        {
            printf("You are not eligible to vote\n");
        }
        else
        {
            printf("You are eligible to vote\n");
        }

        printf("\n");
        free(x);
    }
}
