<template>
  <div class="modal" @click="closeDialog">
    <div class="modal-content" @click.stop>
      <div class="modal-header">
        <h3>添加密码</h3>
        <button @click="closeDialog" class="close-btn">
          <i class="fas fa-times"></i>
        </button>
      </div>
      
      <form @submit.prevent="savePassword">
        <div class="form-group">
          <label for="site-name">网站名称 *</label>
          <input 
            type="text" 
            id="site-name"
            v-model="formData.site_name"
            placeholder="例如：公司内网"
            required
            @input="validateForm"
          />
        </div>
        
        <div class="form-group">
          <label for="site-url">网站/IP地址 *</label>
          <input 
            type="text" 
            id="site-url"
            v-model="formData.site_url"
            placeholder="例如：https://internal.company.com 或 192.168.1.100"
            required
            @input="validateForm"
          />
        </div>
        
        <div class="form-group">
          <label for="username">用户名/账号 *</label>
          <input 
            type="text" 
            id="username"
            v-model="formData.username"
            placeholder="例如：zhangsan@company.com"
            required
            @input="validateForm"
          />
        </div>
        
        <div class="form-group">
          <label for="password">密码 *</label>
          <div class="password-input-container">
            <input 
              :type="showPassword ? 'text' : 'password'"
              id="password"
              v-model="formData.password"
              placeholder="输入密码"
              required
              @input="validateForm"
            />
            <button type="button" @click="togglePasswordVisibility" class="toggle-password">
              <i :class="showPassword ? 'fas fa-eye-slash' : 'fas fa-eye'"></i>
            </button>
            <button type="button" @click="generatePassword" class="generate-password" title="生成强密码">
              <i class="fas fa-dice"></i>
            </button>
          </div>
          
          <div class="password-strength">
            <div class="strength-bar">
              <div class="strength-fill" :class="passwordStrengthClass"></div>
            </div>
            <span class="strength-text">{{ passwordStrengthText }}</span>
          </div>
        </div>
        
        <div class="form-group">
          <label for="notes">备注（可选）</label>
          <textarea 
            id="notes"
            v-model="formData.notes"
            placeholder="添加备注信息..."
            rows="3"
            @input="validateForm"
          />
        </div>
        
        <div class="form-group">
          <label for="tags">标签（可选）</label>
          <input 
            type="text" 
            id="tags"
            v-model="tagInput"
            placeholder="输入标签，按回车添加"
            @keyup.enter="addTag"
          />
          <div class="tags-list">
            <span v-for="tag in formData.tags" :key="tag" class="tag">
              {{ tag }}
              <button type="button" @click="removeTag(tag)" class="tag-remove">
                <i class="fas fa-times"></i>
              </button>
            </span>
          </div>
        </div>
        
        <div class="password-options">
          <div class="form-group">
            <label>
              <input type="checkbox" v-model="formData.generatePassword" @change="handleGeneratePasswordChange">
              自动生成密码
            </label>
          </div>
          
          <div v-if="formData.generatePassword" class="password-options-group">
            <div class="form-group">
              <label for="password-length">密码长度</label>
              <select id="password-length" v-model="passwordOptions.length" @change="generatePassword">
                <option value="12">12 位</option>
                <option value="16">16 位</option>
                <option value="20">20 位</option>
                <option value="24">24 位</option>
              </select>
            </div>
            
            <div class="form-group">
              <label>
                <input type="checkbox" v-model="passwordOptions.includeUppercase">
                包含大写字母
              </label>
            </div>
            
            <div class="form-group">
              <label>
                <input type="checkbox" v-model="passwordOptions.includeLowercase">
                包含小写字母
              </label>
            </div>
            
            <div class="form-group">
              <label>
                <input type="checkbox" v-model="passwordOptions.includeNumbers">
                包含数字
              </label>
            </div>
            
            <div class="form-group">
              <label>
                <input type="checkbox" v-model="passwordOptions.includeSpecial">
                包含特殊字符
              </label>
            </div>
          </div>
        </div>
        
        <div class="modal-footer">
          <button type="button" @click="closeDialog" class="btn btn-secondary">取消</button>
          <button type="submit" class="btn btn-primary" :disabled="!isFormValid">
            <i v-if="saving" class="fas fa-spinner fa-spin"></i>
            {{ saving ? '保存中...' : '保存密码' }}
          </button>
        </div>
      </form>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, nextTick } from 'vue'

const emit = defineEmits(['close', 'saved'])

const props = defineProps({
  existingPassword: {
    type: Object,
    default: null
  }
})

// 响应式数据
const showPassword = ref(false)
const saving = ref(false)
const tagInput = ref('')
const formData = ref({
  site_name: '',
  site_url: '',
  username: '',
  password: '',
  notes: '',
  tags: [],
  generatePassword: false
})

const passwordOptions = ref({
  length: 16,
  includeUppercase: true,
  includeLowercase: true,
  includeNumbers: true,
  includeSpecial: true
})

// 计算属性
const isFormValid = computed(() => {
  return formData.value.site_name && 
         formData.value.site_url && 
         formData.value.username && 
         formData.value.password &&
         !saving.value
})

const passwordStrength = computed(() => {
  const password = formData.value.password
  let strength = 0
  
  // 长度评分
  strength += Math.min(password.length / 4, 25)
  
  // 复杂度评分
  if (password.match(/[a-z]/)) strength += 15
  if (password.match(/[A-Z]/)) strength += 15
  if (password.match(/[0-9]/)) strength += 15
  if (password.match(/[^a-zA-Z0-9]/)) strength += 25
  
  // 检查常见密码
  const commonPasswords = [
    'password', '123456', 'qwerty', 'letmein', 'welcome',
    'admin', 'password1', '12345678', 'abc123', 'password123'
  ]
  
  if (commonPasswords.includes(password.toLowerCase())) {
    strength = 0
  }
  
  return Math.min(strength, 100)
})

const passwordStrengthClass = computed(() => {
  if (passwordStrength.value < 30) return 'strength-weak'
  if (passwordStrength.value < 70) return 'strength-medium'
  return 'strength-strong'
})

const passwordStrengthText = computed(() => {
  if (passwordStrength.value < 30) return '弱'
  if (passwordStrength.value < 70) return '中'
  return '强'
})

// 方法
const togglePasswordVisibility = () => {
  showPassword.value = !showPassword.value
}

const validateForm = () => {
  // 表单验证逻辑
  if (formData.value.generatePassword) {
    generatePassword()
  }
}

const addTag = () => {
  if (tagInput.value.trim() && !formData.value.tags.includes(tagInput.value.trim())) {
    formData.value.tags.push(tagInput.value.trim())
    tagInput.value = ''
  }
}

const removeTag = (tag) => {
  formData.value.tags = formData.value.tags.filter(t => t !== tag)
}

const handleGeneratePasswordChange = () => {
  if (formData.value.generatePassword) {
    generatePassword()
  } else {
    formData.value.password = ''
  }
}

const generatePassword = async () => {
  if (!formData.value.generatePassword) return
  
  try {
    const chars = {
      uppercase: 'ABCDEFGHIJKLMNOPQRSTUVWXYZ',
      lowercase: 'abcdefghijklmnopqrstuvwxyz',
      numbers: '0123456789',
      special: '!@#$%^&*()_+-=[]{}|;:,.<>?'
    }
    
    let charset = ''
    if (passwordOptions.value.includeUppercase) charset += chars.uppercase
    if (passwordOptions.value.includeLowercase) charset += chars.lowercase
    if (passwordOptions.value.includeNumbers) charset += chars.numbers
    if (passwordOptions.value.includeSpecial) charset += chars.special
    
    if (charset === '') {
      alert('请至少选择一种字符类型')
      return
    }
    
    let password = ''
    const length = parseInt(passwordOptions.value.length)
    
    // 生成密码
    for (let i = 0; i < length; i++) {
      const randomIndex = Math.floor(Math.random() * charset.length)
      password += charset[randomIndex]
    }
    
    // 确保包含所有要求的字符类型
    if (passwordOptions.value.includeUppercase && !password.match(/[A-Z]/)) {
      password = password[0] + chars.uppercase[Math.floor(Math.random() * chars.uppercase.length)] + password.slice(1)
    }
    if (passwordOptions.value.includeLowercase && !password.match(/[a-z]/)) {
      password = password[0] + chars.lowercase[Math.floor(Math.random() * chars.lowercase.length)] + password.slice(1)
    }
    if (passwordOptions.value.includeNumbers && !password.match(/[0-9]/)) {
      password = password[0] + chars.numbers[Math.floor(Math.random() * chars.numbers.length)] + password.slice(1)
    }
    if (passwordOptions.value.includeSpecial && !password.match(/[^a-zA-Z0-9]/)) {
      password = password[0] + chars.special[Math.floor(Math.random() * chars.special.length)] + password.slice(1)
    }
    
    formData.value.password = password
  } catch (error) {
    console.error('生成密码失败:', error)
  }
}

const savePassword = async () => {
  if (!isFormValid.value) return
  
  saving.value = true
  
  try {
    // 这里应该调用实际的保存API
    // await axios.post('/api/add-password', formData.value)
    
    // 模拟保存延迟
    await new Promise(resolve => setTimeout(resolve, 1000))
    
    emit('saved', formData.value)
    closeDialog()
  } catch (error) {
    console.error('保存密码失败:', error)
    alert('保存密码失败')
  } finally {
    saving.value = false
  }
}

const closeDialog = () => {
  emit('close')
}

const resetForm = () => {
  formData.value = {
    site_name: '',
    site_url: '',
    username: '',
    password: '',
    notes: '',
    tags: [],
    generatePassword: false
  }
  tagInput.value = ''
  passwordOptions.value = {
    length: 16,
    includeUppercase: true,
    includeLowercase: true,
    includeNumbers: true,
    includeSpecial: true
  }
  showPassword.value = false
  saving.value = false
}

// 监听密码输入变化
const updatePasswordStrength = () => {
  // 密码强度计算已经在计算属性中处理
}

// 生命周期
onMounted(() => {
  resetForm()
  
  // 如果有传入的现有密码，进行编辑模式
  if (props.existingPassword) {
    formData.value = { ...props.existingPassword }
  }
  
  // 监听密码输入变化
  nextTick(() => {
    const passwordInput = document.getElementById('password')
    if (passwordInput) {
      passwordInput.addEventListener('input', updatePasswordStrength)
    }
  })
})

// 暴露方法给父组件
defineExpose({
  resetForm,
  generatePassword
})
</script>

<style scoped>
.modal {
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background: rgba(0, 0, 0, 0.5);
  z-index: 1000;
  display: flex;
  align-items: center;
  justify-content: center;
}

.modal-content {
  background: white;
  border-radius: 12px;
  padding: 30px;
  max-width: 600px;
  width: 90%;
  max-height: 80vh;
  overflow-y: auto;
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.3);
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 25px;
  padding-bottom: 15px;
  border-bottom: 1px solid #e1e5e9;
}

.modal-header h3 {
  color: #333;
  font-size: 1.5rem;
  margin: 0;
}

.close-btn {
  background: none;
  border: none;
  font-size: 1.5rem;
  cursor: pointer;
  color: #666;
  padding: 5px;
  border-radius: 50%;
  transition: all 0.3s;
}

.close-btn:hover {
  color: #333;
  background: #f8f9fa;
}

.form-group {
  margin-bottom: 20px;
}

.form-group label {
  display: block;
  margin-bottom: 8px;
  font-weight: 500;
  color: #555;
}

.form-group input,
.form-group select,
.form-group textarea {
  width: 100%;
  padding: 12px;
  border: 2px solid #e1e5e9;
  border-radius: 8px;
  font-size: 16px;
  transition: border-color 0.3s;
}

.form-group input:focus,
.form-group select:focus,
.form-group textarea:focus {
  outline: none;
  border-color: #667eea;
}

.form-group textarea {
  resize: vertical;
  min-height: 80px;
}

.password-input-container {
  position: relative;
  display: flex;
  align-items: center;
}

.password-input-container input {
  flex: 1;
}

.toggle-password,
.generate-password {
  position: absolute;
  right: 10px;
  background: none;
  border: none;
  cursor: pointer;
  padding: 5px;
  border-radius: 4px;
  transition: background-color 0.3s;
}

.toggle-password:hover,
.generate-password:hover {
  background: #f8f9fa;
}

.password-strength {
  margin-top: 10px;
}

.strength-bar {
  height: 4px;
  background: #e1e5e9;
  border-radius: 2px;
  overflow: hidden;
  margin-bottom: 5px;
}

.strength-fill {
  height: 100%;
  transition: all 0.3s;
}

.strength-weak {
  background: #dc3545;
}

.strength-medium {
  background: #ffc107;
}

.strength-strong {
  background: #28a745;
}

.strength-text {
  font-size: 0.85rem;
  color: #666;
}

.tags-list {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 10px;
}

.tag {
  background: #e9ecef;
  padding: 4px 12px;
  border-radius: 16px;
  font-size: 0.85rem;
  color: #495057;
  display: flex;
  align-items: center;
  gap: 5px;
}

.tag-remove {
  background: none;
  border: none;
  cursor: pointer;
  color: #6c757d;
  padding: 0;
  font-size: 0.8rem;
}

.tag-remove:hover {
  color: #dc3545;
}

.password-options {
  margin-top: 25px;
  padding-top: 25px;
  border-top: 1px solid #e1e5e9;
}

.password-options-group {
  margin-top: 20px;
}

.password-options-group .form-group {
  margin-bottom: 15px;
}

.password-options-group .form-group label {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
}

.password-options-group .form-group input[type="checkbox"] {
  width: auto;
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  margin-top: 30px;
  padding-top: 20px;
  border-top: 1px solid #e1e5e9;
}

.btn {
  padding: 12px 24px;
  border: none;
  border-radius: 8px;
  font-size: 16px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.3s;
  display: flex;
  align-items: center;
  gap: 8px;
}

.btn-primary {
  background: #667eea;
  color: white;
}

.btn-primary:hover {
  background: #5a6fd8;
}

.btn-secondary {
  background: #6c757d;
  color: white;
}

.btn-secondary:hover {
  background: #5a6268;
}

.btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

@media (max-width: 768px) {
  .modal-content {
    width: 95%;
    margin: 20px;
  }
  
  .modal-footer {
    flex-direction: column;
  }
  
  .btn {
    width: 100%;
    justify-content: center;
  }
}
</style>