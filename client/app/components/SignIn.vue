<template>
  <div class="flex min-h-dvh items-center justify-center p-4">
    <UCard class="w-full max-w-md">
      <template #header>
        <h1 class="text-xl font-semibold">{{ t("face.signin.title") }}</h1>
      </template>

      <form id="sign-in" class="flex flex-col gap-4" @submit.prevent="submit">
        <UFormField :label="t('face.signin.name')" name="name">
          <UInput v-model="name" class="w-full" autocomplete="username" required />
        </UFormField>
        <UFormField :label="t('face.signin.password')" name="password">
          <UInput v-model="password" class="w-full" type="password" autocomplete="current-password" required />
        </UFormField>
        <UFormField :label="t('face.signin.device')" name="device">
          <UInput v-model="device" class="w-full" required />
        </UFormField>
        <p v-if="refusal" class="text-sm text-error">{{ refusal }}</p>
      </form>

      <template #footer>
        <div class="flex justify-end">
          <UButton type="submit" form="sign-in" :loading="busy" :label="t('face.signin.submit')" />
        </div>
      </template>
    </UCard>
  </div>
</template>

<script setup lang="ts">
// Signing this screen in with a BMO account, once (docs/face.md, 「How a screen signs in」). The name
// given here is the one the screen stands under on BMO's account screen, where it is revoked.

const { t } = useI18n();
const { signIn } = useDevice();

const name = ref("");
const password = ref("");
const device = ref(t("face.signin.device_default"));
const busy = ref(false);
const refusal = ref("");

async function submit(): Promise<void> {
  busy.value = true;
  refusal.value = "";

  const result = await signIn(name.value.trim(), password.value, device.value.trim());

  busy.value = false;

  if (!result.ok) {
    refusal.value = t(`face.signin.${result.reason}`);
    password.value = "";
  }
}
</script>
