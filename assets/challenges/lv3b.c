#include <stdio.h>
#include <string.h>
int main(void){ char buf[32]; unsigned char out[32]; int n;
  printf("password: "); scanf("%31s", buf); n = strlen(buf);
  if (n < 4) { puts("Wrong"); return 1; }
  for (int i = 0; i < n; i++) out[i] = buf[i] ^ (0x42 + i);
  const unsigned char target[8] = {0x10,0x26,0x32,0x20,0x34,0x34,0x7b,0x68};
  if (n == 8 && !memcmp(out, target, 8)) { puts("Correct! FLAG{lv3b_xorpos}"); return 0; }
  puts("Wrong"); return 1; }