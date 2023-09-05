import os.path

from aws_cdk.aws_s3_assets import Asset

from constructs import Construct

from aws_cdk import (
    aws_ec2 as ec2,
    aws_iam as iam,
    aws_ssm as ssm,
    App,
    Stack,
    CfnOutput,
)

from dotenv import load_dotenv

load_dotenv()

SSH_PUBLIC_KEY = os.getenv("SSH_PUBLIC_KEY")

dirname = os.path.dirname(__file__)


class EC2InstanceStack(Stack):
    def __init__(self, scope: Construct, id: str, **kwargs) -> None:
        super().__init__(scope, id, **kwargs)

        # VPC
        vpc = ec2.Vpc(
            self,
            "VPC",
            nat_gateways=0,
            subnet_configuration=[
                ec2.SubnetConfiguration(
                    name="public", subnet_type=ec2.SubnetType.PUBLIC
                )
            ],
        )

        # Instance Role and SSM Managed Policy
        role = iam.Role(
            self, "InstanceSSM", assumed_by=iam.ServicePrincipal("ec2.amazonaws.com")
        )

        role.add_managed_policy(
            iam.ManagedPolicy.from_aws_managed_policy_name(
                "AmazonSSMManagedInstanceCore"
            )
        )

        # Security group for Ubuntu Usershell
        sg_ubuntu = ec2.SecurityGroup(
            self,
            "SGUbuntuUserShell",
            vpc=vpc,
            allow_all_outbound=True,
            description="Security group for Ubuntu Usershell instance",
        )

        # Allow Inbound SSH and Anvil
        sg_ubuntu.add_ingress_rule(ec2.Peer.any_ipv4(), ec2.Port.tcp(8545))
        sg_ubuntu.add_ingress_rule(ec2.Peer.any_ipv4(), ec2.Port.tcp(22))

        # Ubuntu AMI
        # https://discourse.ubuntu.com/t/finding-ubuntu-images-with-the-aws-ssm-parameter-store/15507
        ubuntu_ami = ec2.MachineImage.from_ssm_parameter(
            "/aws/service/canonical/ubuntu/server/focal/stable/current/amd64/hvm/ebs-gp2/ami-id"
        )

        # create new keypair
        key = ec2.CfnKeyPair(
            self,
            "KeyPair",
            key_name="ubuntu-ssh-key",
            public_key_material=SSH_PUBLIC_KEY,
        )

        # Ubuntu Usershell Instance
        ubuntu_ec2 = ec2.Instance(
            self,
            "ubuntu_ec2",
            instance_type=ec2.InstanceType("t2.large"),
            machine_image=ubuntu_ami,
            vpc=vpc,
            role=role,
            security_group=sg_ubuntu,
            instance_name="Ubuntu Usershell",
            block_devices=[
                ec2.BlockDevice(
                    device_name="/dev/sda1", volume=ec2.BlockDeviceVolume.ebs(200)
                )
            ],
            key_name="ubuntu-ssh-key",  # use the public key
        )

        # Install AWS CLI
        ssm.CfnAssociation(
            self,
            "SSMAssociation",
            name="AWS-RunShellScript",
            targets=[{"key": "InstanceIds", "values": [ubuntu_ec2.instance_id]}],
            parameters={
                "commands": [
                    # set hostname
                    "hostnamectl set-hostname ubuntu",
                    "echo '127.0.0.1 ubuntu' >> /etc/hosts",
                    # update and install packages
                    "apt-get update",
                    "apt-get install -y awscli docker.io protobuf-compiler libprotobuf-dev pkg-config libssh-dev libssl-dev pkg-config build-essential neovim git net-tools netcat",
                    # setup docker
                    "usermod -a -G docker ubuntu",
                    "chmod 666 /var/run/docker.sock",
                    "systemctl enable docker.service",
                    "systemctl start docker.service",
                    # install rust as the ubuntu user
                    "su ubuntu -c 'curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y'",
                    "echo 'export PATH=$PATH:/home/ubuntu/.cargo/bin' >> /home/ubuntu/.bashrc",
                    # Clone BLS-TSS-Network code
                    "cd /tmp",
                    "git clone https://github.com/ARPA-Network/BLS-TSS-Network.git",
                    "chown -R ubuntu:ubuntu /tmp/BLS-TSS-Network",
                    # create complete file
                    "touch /tmp/complete",
                    # broadcast completion wiht wall
                    "wall 'SSM Commands Complete!'",
                ]
            },
        )

        # Useful Outputs
        useful_outputs = CfnOutput(
            self,
            "UsefulOutputs",
            value=f"""
aws ssm start-session --target {ubuntu_ec2.instance_id}
ssh ubuntu@{ubuntu_ec2.instance_public_ip}
""",
            description="Useful Outputs",
        )


app = App()
EC2InstanceStack(app, "usershell-ec2-stack")

app.synth()
