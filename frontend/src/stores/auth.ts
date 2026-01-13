import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import api from '../api'

export const useAuthStore = defineStore('auth', () => {
  const token = ref<string | null>(localStorage.getItem('token'))
  const username = ref<string | null>(localStorage.getItem('username'))

  const isAuthenticated = computed(() => !!token.value)

  async function login(user: string, password: string) {
    const response = await api.post('/api/auth/login', { username: user, password })
    token.value = response.data.token
    username.value = user
    localStorage.setItem('token', response.data.token)
    localStorage.setItem('username', user)
  }

  function logout() {
    token.value = null
    username.value = null
    localStorage.removeItem('token')
    localStorage.removeItem('username')
  }

  return {
    token,
    username,
    isAuthenticated,
    login,
    logout
  }
})
