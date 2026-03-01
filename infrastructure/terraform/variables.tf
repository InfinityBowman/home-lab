variable "cloudflare_api_token" {
  type      = string
  sensitive = true
}

variable "account_id" {
  type = string
}

variable "zone_id" {
  type = string
}

variable "domain" {
  type    = string
  default = "jacobmaynard.dev"
}

variable "admin_email" {
  type    = string
  default = "jacobamaynard@proton.me"
}
