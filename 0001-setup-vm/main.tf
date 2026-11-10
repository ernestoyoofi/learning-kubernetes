terraform {
  required_providers {
    proxmox = {
      source  = "telmate/proxmox"
      version = "3.0.2-rc07"
    }
  }
}

variable "pm_api_url" {
  type        = string
  description = "Proxmox API URL"
}

variable "pm_api_token_id" {
  type        = string
  description = "Proxmox API Token ID"
}

variable "pm_api_token_secret" {
  type        = string
  description = "Proxmox API Token Secret"
  sensitive   = true
}

variable "target_node" {
  type        = string
  description = "Proxmox Node Target"
}

variable "clone_templatevm" {
  type        = string
  description = "Proxmox Template VM"
}

provider "proxmox" {
  pm_api_url          = var.pm_api_url
  pm_api_token_id     = var.pm_api_token_id
  pm_api_token_secret = var.pm_api_token_secret
  pm_tls_insecure     = true
}

variable "vm_count" {
  default = 3
}

resource "proxmox_vm_qemu" "amazon_linux_vms" {
  count       = var.vm_count
  name        = "vm-amazonlinux-0${count.index + 1}"
  target_node = var.target_node
  clone       = var.clone_templatevm

  cores   = 2
  sockets = 1
  cpu     = "host"
  memory  = 2048
  scsihw  = "virtio-scsi-pci"
  boot    = "order=scsi0"

  disk {
    slot     = 0
    type     = "scsi"
    storage  = "local-lvm"
    size     = "64G"
    format   = "raw"
    iothread = 1
  }

  network {
    model  = "virtio"
    bridge = "vlan_30"
  }

  os_type    = "cloud-init"
  ipconfig0  = "ip=192.168.30.5${count.index + 1}/24,gw=192.168.30.1"
  ciuser     = "ec2-user"
  sshkeys    = file("ssh-file.pub")

  lifecycle {
    ignore_changes = [
      network,
      disk,
    ]
  }
}

output "vm_ips" {
  value = [for index, vm in proxmox_vm_qemu.amazon_linux_vms : "vm-amazonlinux-0${index + 1} -> IP: 192.168.30.5${index + 1}"]
}