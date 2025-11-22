#include <stdio.h>
#include <unistd.h>

int main() { printf("Hello %d", getgid()); }
