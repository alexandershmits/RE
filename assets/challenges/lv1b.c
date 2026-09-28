#include <stdio.h>
int main(void){ unsigned x;
  printf("code: "); scanf("%u", &x);
  if (x == 31337) { puts("Correct! FLAG{lv1b_31337}"); return 0; }
  puts("Wrong"); return 1; }