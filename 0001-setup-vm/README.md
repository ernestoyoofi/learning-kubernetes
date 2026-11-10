# Setting Up Virtual Machines

The first step is to create the virtual machines (VMs) that will be used in this lab. This example creates three VMs, each with 2 vCPUs and 2 GB of memory. You can change the configuration in [`main.tf`](./main.tf) and the Proxmox connection settings in [`terraform.tfvars`](./terraform.tfvars).

> [!NOTE]
> Terraform is an Infrastructure as Code (IaC) tool. It can automate the creation and management of infrastructure, including virtual machines.

## Create a Proxmox Template

The Terraform configuration clones a Proxmox VM template, so create the template before running Terraform.

This example uses Amazon Linux. Find an Amazon Linux KVM image in the [Amazon Linux documentation](https://docs.aws.amazon.com/linux/al2023/ug/amazon-linux-2023-virtual-machine.html). Proxmox systems with x86 processors generally use the `kvm` image; use `kvm-arm64` for ARM64 systems. Download the image whose filename ends in `.gpt.qcow2`.

![Search for Amazon Linux KVM images and open the AWS documentation](../assets/setup-vm-1-search-amazon-linux.png)

![Find the Amazon Linux download location](../assets/setup-vm-2-find-on-documentation.png)

![Choose the KVM image for your processor architecture](../assets/setup-vm-3-find-specifict-at-kvm.png)

![Download the Amazon Linux KVM image](../assets/setup-vm-4-download-file-al2023-kvm.png)

Copy the image URL and download it to your Proxmox server. For example, open the Proxmox shell, check the current directory with `pwd`, and use `wget` with the copied URL. The ISO template directory, `/var/lib/vz/template/iso/`, is a suggested location.

Create a VM using the downloaded image and configure it for your environment. For this example, use the `q35` machine type and add an EFI disk, as the Amazon Linux image uses UEFI. Configure the CPU and other settings as needed, and make sure the VM is configured to work with cloud-init. Once the VM is ready, convert it to a template. The following recording shows an example of creating a VM template:

https://github.com/user-attachments/assets/b719f901-d499-41ee-aef8-d59d9b5df305

> [!NOTE]
> If your Proxmox storage uses an SSD, you can check the **SSD Emulation** and **Discard** boxes, provided you are using a SATA/NVMe/M.2 SSD.

The template's Proxmox name must match the `clone_templatevm` value in `terraform.tfvars`.

## Prepare the Terraform Configuration

Run the following commands from the `setup-vm` directory.

1. Copy `terraform.tfvars.example` to `terraform.tfvars` if you have not created the file already.
2. Edit `terraform.tfvars` and provide your Proxmox API URL, API token ID and secret, target node, and template name. The values in the example file are placeholders; replace them with values for your Proxmox environment.
3. Place an SSH **public** key named `ssh-file.pub` in this directory. The configuration reads this file and adds the key to the VMs for the `ec2-user` account. Do not put a private key in this file or commit private keys or real API secrets to source control.

Review [`main.tf`](./main.tf) before applying it. It defines the Proxmox provider and its input variables, then creates `vm_count` cloned VMs. The default count is three. Each VM is configured with 2 CPU cores, 2048 MB of memory, a 64 GB disk, and an IP address in the `192.168.30.0/24` subnet. Adjust the node, network bridge, storage, IP range, and gateway in the configuration if they do not match your Proxmox environment. The `vm_ips` output lists the configured VM names and IP addresses.

## Initialize, Review, and Apply

Make sure Terraform is installed and that your workstation can reach the Proxmox API. From the `setup-vm` directory, initialize the working directory to download the required provider:

```shell
terraform init
```

Check the configuration syntax and formatting:

```shell
terraform fmt -check
terraform validate
```

Create and review an execution plan before making changes:

```shell
terraform plan -out=tfplan
```

If the plan shows the VMs and settings you expect, apply that exact plan:

```shell
terraform apply tfplan
```

Terraform displays the `vm_ips` output after a successful apply. Review the plan carefully before applying: it creates VMs on your Proxmox environment. The saved plan file may contain sensitive values, so do not commit it to source control.

To preview the changes Terraform would make later, run `terraform plan` again. To remove the VMs managed by this configuration, run `terraform destroy` from this directory and confirm the proposed changes before proceeding.

## Set Up NFS Storage

This section uses an additional 80 GB SSD partition (`/dev/sdb2`) on the Proxmox 8.4 host as an NFS share for the Amazon Linux 2023 VMs.

> [!WARNING]
> Formatting `/dev/sdb2` erases its contents. Confirm that it is the intended 80 GB partition before running `mkfs`.

### Configure the Proxmox NFS Server

Run these commands in the Proxmox shell. First, verify the disk and partition:

```shell
lsblk -o NAME,SIZE,FSTYPE,UUID,MOUNTPOINTS
```

Confirm that `/dev/sdb2` is the additional 80 GB partition and that it does not contain data you need. Format it as ext4:

```shell
mkfs.ext4 /dev/sdb2
```

Create a mount point and get the partition UUID:

```shell
mkdir -p /srv/nfs/kube-storage
blkid /dev/sdb2
```

Add the UUID reported by `blkid` to `/etc/fstab` so the partition mounts after a Proxmox reboot:

```fstab
UUID=<sdb2-uuid> /srv/nfs/kube-storage ext4 defaults 0 2
```

Replace `<sdb2-uuid>` with the actual UUID, then mount and verify it:

```shell
mount -a
findmnt /srv/nfs/kube-storage
```

Install and enable the NFS server:

```shell
apt update
apt install nfs-kernel-server
systemctl enable --now nfs-kernel-server
```

Export the storage to the VM network. The example below uses `192.168.30.0/24`; replace it with the subnet containing your Amazon Linux VMs. Keep the export restricted to trusted clients.

Add this line to `/etc/exports`:

```exports
/srv/nfs/kube-storage 192.168.30.0/24(rw,sync,no_subtree_check,root_squash)
```

Apply and verify the export:

```shell
exportfs -rav
exportfs -v
```

Allow inbound TCP port `2049` from the VM subnet in the Proxmox firewall and any other firewall between the VMs and the host. This guide uses NFSv4.2, which does not require opening the additional NFSv3 RPC ports.

For a basic write test as `ec2-user`, make the directory writable by that account. Check its numeric IDs on a VM with `id ec2-user`, then use those IDs on Proxmox (Amazon Linux's default `ec2-user` is usually `1000:1000`):

```shell
chown 1000:1000 /srv/nfs/kube-storage
chmod 0755 /srv/nfs/kube-storage
```

Adjust ownership and permissions to match the users or Kubernetes workloads that will use the share. With `root_squash` enabled, client root does not have root privileges on the NFS server.

### Mount the Share on Amazon Linux 2023

Run these commands on each Amazon Linux 2023 VM. Replace `<proxmox-ip>` with the Proxmox host's reachable IP address:

```shell
sudo dnf install -y nfs-utils
sudo mkdir -p /mnt/kube-storage
sudo mount -t nfs -o nfsvers=4.2 <proxmox-ip>:/srv/nfs/kube-storage /mnt/kube-storage
```

Verify the mount and test writing as `ec2-user`:

```shell
findmnt /mnt/kube-storage
touch /mnt/kube-storage/nfs-test
rm /mnt/kube-storage/nfs-test
```

To mount the share automatically after reboot, add this line to `/etc/fstab` on each VM:

```fstab
<proxmox-ip>:/srv/nfs/kube-storage /mnt/kube-storage nfs4 nfsvers=4.2,_netdev,nofail,hard,timeo=600,retrans=2 0 0
```

Replace `<proxmox-ip>` with the same Proxmox IP address. `_netdev` marks this as a network mount; `nofail` lets the VM finish booting if the NFS server is temporarily unavailable. Test the entry before rebooting:

```shell
sudo umount /mnt/kube-storage
sudo mount -a
findmnt /mnt/kube-storage
```
