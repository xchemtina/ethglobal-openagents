'use client'

import { useAccount, useConnect, useDisconnect } from 'wagmi'
import { useState } from 'react'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { cn } from '@/lib/utils'
import { Wallet, Copy, ExternalLink, LogOut, ChevronDown, Check } from 'lucide-react'

function truncateAddress(address: string) {
  return `${address.slice(0, 6)}...${address.slice(-4)}`
}

export function ConnectWalletButton({ className }: { className?: string }) {
  const { address, isConnected, chain } = useAccount()
  const { connect, connectors, isPending } = useConnect()
  const { disconnect } = useDisconnect()
  const [copied, setCopied] = useState(false)
  const [isOpen, setIsOpen] = useState(false)

  const copyAddress = async () => {
    if (address) {
      await navigator.clipboard.writeText(address)
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    }
  }

  const openEtherscan = () => {
    if (address && chain) {
      const baseUrl = chain.id === 1 
        ? 'https://etherscan.io' 
        : 'https://sepolia.etherscan.io'
      window.open(`${baseUrl}/address/${address}`, '_blank')
    }
  }

  if (isConnected && address) {
    return (
      <DropdownMenu open={isOpen} onOpenChange={setIsOpen}>
        <DropdownMenuTrigger asChild>
          <Button
            variant="outline"
            className={cn(
              'gap-2 border-primary/30 bg-primary/5 hover:bg-primary/10 hover:border-primary/50 font-mono',
              className
            )}
          >
            <span className="size-2 rounded-full bg-emerald-500 animate-pulse" />
            {truncateAddress(address)}
            <ChevronDown className="size-4 text-muted-foreground" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end" className="w-56">
          <DropdownMenuLabel className="font-mono text-xs text-muted-foreground">
            {chain?.name || 'Unknown Network'}
          </DropdownMenuLabel>
          <DropdownMenuSeparator />
          <DropdownMenuItem onClick={copyAddress}>
            {copied ? <Check className="text-emerald-500" /> : <Copy />}
            {copied ? 'Copied!' : 'Copy Address'}
          </DropdownMenuItem>
          <DropdownMenuItem onClick={openEtherscan}>
            <ExternalLink />
            View on Etherscan
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem
            onClick={() => disconnect()}
            variant="destructive"
          >
            <LogOut />
            Disconnect
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    )
  }

  return (
    <DropdownMenu open={isOpen} onOpenChange={setIsOpen}>
      <DropdownMenuTrigger asChild>
        <Button
          variant="outline"
          className={cn(
            'gap-2 border-primary/30 hover:bg-primary/10 hover:border-primary/50',
            className
          )}
          disabled={isPending}
        >
          <Wallet className="size-4" />
          {isPending ? 'Connecting...' : 'Connect Wallet'}
          <ChevronDown className="size-4 text-muted-foreground" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-56">
        <DropdownMenuLabel>Select Wallet</DropdownMenuLabel>
        <DropdownMenuSeparator />
        {connectors.map((connector) => (
          <DropdownMenuItem
            key={connector.uid}
            onClick={() => connect({ connector })}
          >
            <WalletIcon type={connector.name} />
            {connector.name}
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  )
}

function WalletIcon({ type }: { type: string }) {
  // Simple icon based on connector name
  const iconClass = 'size-4'
  
  if (type.toLowerCase().includes('metamask')) {
    return (
      <svg className={iconClass} viewBox="0 0 24 24" fill="none">
        <path d="M21.5 6L13 2.5L4.5 6L8 8L13 6L18 8L21.5 6Z" fill="#E17726" stroke="#E17726" strokeLinecap="round" strokeLinejoin="round"/>
        <path d="M4.5 6V15L8 17V8L4.5 6Z" fill="#E27625" stroke="#E27625" strokeLinecap="round" strokeLinejoin="round"/>
        <path d="M21.5 6V15L18 17V8L21.5 6Z" fill="#E27625" stroke="#E27625" strokeLinecap="round" strokeLinejoin="round"/>
        <path d="M8 17L13 21.5L18 17L13 15L8 17Z" fill="#D5BFB2" stroke="#D5BFB2" strokeLinecap="round" strokeLinejoin="round"/>
      </svg>
    )
  }
  
  if (type.toLowerCase().includes('coinbase')) {
    return (
      <svg className={iconClass} viewBox="0 0 24 24" fill="none">
        <circle cx="12" cy="12" r="10" fill="#0052FF"/>
        <path d="M12 6C8.68629 6 6 8.68629 6 12C6 15.3137 8.68629 18 12 18C15.3137 18 18 15.3137 18 12C18 8.68629 15.3137 6 12 6Z" fill="white"/>
        <path d="M10.5 10.5H13.5V13.5H10.5V10.5Z" fill="#0052FF"/>
      </svg>
    )
  }
  
  // Default wallet icon
  return <Wallet className={iconClass} />
}
