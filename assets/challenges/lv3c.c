#include <stdio.h>
int main(void){ unsigned x;
  printf("number: "); scanf("%u", &x);
  unsigned h = 5381;
  for (int i = 0; i < 4; i++) { h = h * 33 + (x & 0xFF); x >>= 8; }
  if (h == 2086954227u) { puts("Correct! FLAG{lv3c_djb2}"); return 0; }
  puts("Wrong"); return 1; }