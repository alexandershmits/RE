// ch-dotnet: IL-подобная стековая виртуальная машина (имитация .NET)
#include <stdio.h>
#include <string.h>

static const unsigned char il[] = {
  0x0C, 0x11, 0x0C, 0x14, 0x0C, 0x53, 0x0C, 0x4C, 0x0C, 0x0C, 0x0C, 0x43, 0x0C, 0x3B, 0x0C, 0x01
};

int main(void) {
  char buf[32];
  printf("password: ");
  scanf("%31s", buf);
  unsigned stack[16]; int sp = 0;
  for (int i = 0; i < 8; i++) {
    unsigned v = (unsigned char)buf[i];
    v = v ^ il[2*i];
    v = v - il[2*i+1];
    stack[sp++] = v;
  }
  for (int i = 0; i < 8; i++) {
    if (stack[i] != 0x2C) { puts("Verification failed"); return 1; }
  }
  puts("Correct! FLAG{il_st4ck_vm}");
  return 0;
}
